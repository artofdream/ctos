//! Minimal EL0 SVC ABI (Track A / A1 / ADR-021).
//!
//! Public numbers start at 16 so they do not collide with the ADR-013
//! probe immediates (`SVC #0` first mile, `#1` standing, `#2` restore).
//! A6 adds 19–23 (thin VFS / memfs, ADR-027).
//! Track N / N4 adds 24–25 (net MAC + ICMP ping, ADR-068).
//! Track N / N3.x adds 26 (UDP DNS probe, ADR-071).
//! ADR-075 adds 27 (`fs_mkdir` — FAT16 root directory create via thin VFS).
//! ADR-076 adds 28 (`net_tcp_echo` — quiet thin TCP guestfwd echo).
//! Not Linux. Not POSIX. Not app hosting. Not a TCP/UDP product stack.

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::el0;
use crate::exception::{self, ExceptionContext};
use crate::frame;
use crate::paging;
use crate::uart;
use crate::vfs;
use crate::virtio;

/// `SVC #0` first-mile return (ADR-013). Not a public ABI number.
pub const SVC_PROBE_RETURN: u64 = 0;
/// `SVC #1` standing announce (ADR-013). Not a public ABI number.
#[allow(dead_code)]
pub const SVC_PROBE_STANDING: u64 = 1;
/// `SVC #2` standing restore (ADR-013). Not a public ABI number.
#[allow(dead_code)]
pub const SVC_PROBE_RESTORE: u64 = 2;

/// Halt the EL0 trip and return to the EL1 caller. `x0` = status.
pub const SYS_EXIT: u64 = 16;
/// Write `x1` bytes from user pointer `x0` to the PL011. Returns count.
pub const SYS_UART_WRITE: u64 = 17;
/// Cooperative hint. This mile records the call and returns to EL0.
/// Does **not** run `sched::yield_now` (that path is EL1-only, ADR-010).
pub const SYS_YIELD: u64 = 18;
/// Create an empty memfs path and return a handle (ADR-027).
pub const SYS_FS_CREATE: u64 = 19;
/// Open an existing memfs path (ADR-027).
pub const SYS_FS_OPEN: u64 = 20;
/// Read from a memfs handle into a user buffer (ADR-027).
pub const SYS_FS_READ: u64 = 21;
/// Write a user buffer into a memfs handle (ADR-027).
pub const SYS_FS_WRITE: u64 = 22;
/// Close a memfs handle (ADR-027).
pub const SYS_FS_CLOSE: u64 = 23;
/// Copy guest virtio-net MAC into a user buffer (ADR-068).
pub const SYS_NET_MAC: u64 = 24;
/// Kernel-path ICMP echo to SLIRP gateway (ADR-068).
pub const SYS_NET_PING: u64 = 25;
/// Kernel-path UDP DNS probe vs SLIRP DNS (ADR-071).
pub const SYS_NET_UDP_DNS: u64 = 26;
/// Create a FAT16 root directory via thin VFS (ADR-075).
pub const SYS_FS_MKDIR: u64 = 27;
/// Kernel-path thin TCP echo vs guestfwd (ADR-076).
pub const SYS_NET_TCP_ECHO: u64 = 28;

/// `fs_mkdir` success.
pub const FS_MKDIR_OK: u64 = 0;
/// `fs_mkdir` name already exists (file or directory).
pub const FS_MKDIR_EXISTS: u64 = 1;
/// `fs_mkdir` bad path (grammar / nested / memfs).
pub const FS_MKDIR_BAD_PATH: u64 = 2;

/// Hard cap on `SYS_UART_WRITE`. Longer lengths return 0.
pub const UART_WRITE_MAX: u64 = 64;
/// Hard cap on `SYS_FS_READ` / `SYS_FS_WRITE` copies.
pub const FS_IO_MAX: u64 = 64;
/// Reject value for `SYS_FS_READ` / `SYS_FS_CLOSE` (0 is a valid empty read).
pub const FS_ERR: u64 = u64::MAX;
/// Guest MAC length for `SYS_NET_MAC`.
pub const NET_MAC_LEN: u64 = 6;
/// Reject value for `SYS_NET_PING` / `SYS_NET_UDP_DNS` / `SYS_NET_TCP_ECHO`.
pub const NET_ERR: u64 = u64::MAX;

/// User buffer the ABI probe writes via `SYS_UART_WRITE`.
pub const USER_UART_MSG: &[u8] = b"svc: user-hi\n";

const SVC16_A64: u32 = 0xD4000001 | ((SYS_EXIT as u32) << 5);
const SVC17_A64: u32 = 0xD4000001 | ((SYS_UART_WRITE as u32) << 5);
const SVC18_A64: u32 = 0xD4000001 | ((SYS_YIELD as u32) << 5);
const SVC19_A64: u32 = 0xD4000001 | ((SYS_FS_CREATE as u32) << 5);
const SVC20_A64: u32 = 0xD4000001 | ((SYS_FS_OPEN as u32) << 5);
const SVC21_A64: u32 = 0xD4000001 | ((SYS_FS_READ as u32) << 5);
const SVC22_A64: u32 = 0xD4000001 | ((SYS_FS_WRITE as u32) << 5);
const SVC23_A64: u32 = 0xD4000001 | ((SYS_FS_CLOSE as u32) << 5);
#[cfg(test)]
const SVC24_A64: u32 = 0xD4000001 | ((SYS_NET_MAC as u32) << 5);
#[cfg(test)]
const SVC25_A64: u32 = 0xD4000001 | ((SYS_NET_PING as u32) << 5);
#[cfg(test)]
const SVC26_A64: u32 = 0xD4000001 | ((SYS_NET_UDP_DNS as u32) << 5);
#[cfg(test)]
const SVC27_A64: u32 = 0xD4000001 | ((SYS_FS_MKDIR as u32) << 5);
#[cfg(test)]
const SVC28_A64: u32 = 0xD4000001 | ((SYS_NET_TCP_ECHO as u32) << 5);

const EL0_FS_PATH: &[u8] = b"/eprobe";
const EL0_FS_BYTES: &[u8] = b"memfs-el0";
const EL0_FS_REJECT_PATH: &[u8] = b"/kreject";

static YIELD_COUNT: AtomicU64 = AtomicU64::new(0);
static EXIT_STATUS: AtomicU64 = AtomicU64::new(u64::MAX);
static UART_LAST: AtomicU64 = AtomicU64::new(u64::MAX);
static YIELD_OK: AtomicBool = AtomicBool::new(false);
static UART_OK: AtomicBool = AtomicBool::new(false);
static EXIT_OK: AtomicBool = AtomicBool::new(false);
static FS_CREATE: AtomicU64 = AtomicU64::new(0);
static FS_OPEN: AtomicU64 = AtomicU64::new(0);
static FS_READ: AtomicU64 = AtomicU64::new(u64::MAX);
static FS_WRITE: AtomicU64 = AtomicU64::new(u64::MAX);
static FS_CLOSE_OK: AtomicBool = AtomicBool::new(false);
static NET_MAC_LAST: AtomicU64 = AtomicU64::new(u64::MAX);
static NET_PING_OK: AtomicBool = AtomicBool::new(false);
static NET_UDP_DNS_OK: AtomicBool = AtomicBool::new(false);
static FS_MKDIR_LAST: AtomicU64 = AtomicU64::new(u64::MAX);
static NET_TCP_ECHO_OK: AtomicBool = AtomicBool::new(false);
/// ADR-094: `x0` returned by the last public syscall other than `SYS_EXIT`.
static LAST_SVC_RET: AtomicU64 = AtomicU64::new(u64::MAX);

/// What the lower-EL handler should do after a public ABI call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvcAction {
    StayEl0,
    ReturnEl1,
}

#[cfg(test)]
fn svc_a64(imm: u64) -> u32 {
    0xD4000001 | ((imm as u32) << 5)
}

fn movz(rd: u32, imm16: u16, lsl: u32) -> u32 {
    let hw = lsl / 16;
    0xD2800000 | (hw << 21) | ((imm16 as u32) << 5) | rd
}

fn movk(rd: u32, imm16: u16, lsl: u32) -> u32 {
    let hw = lsl / 16;
    0xF2800000 | (hw << 21) | ((imm16 as u32) << 5) | rd
}

fn sync_icache(ptr: *const u32) {
    let x = ptr as u64;
    unsafe {
        core::arch::asm!(
            "dc cvau, {x}",
            "dsb ish",
            "ic ivau, {x}",
            "dsb ish",
            "isb",
            x = in(reg) x,
            options(nostack, preserves_flags),
        );
    }
}

fn write_word(ptr: *mut u32, insn: u32) {
    unsafe {
        core::ptr::write_volatile(ptr, insn);
    }
    sync_icache(ptr);
}

/// Encode `val` into `Xd` (MOVZ + MOVK as needed). Returns words written.
fn write_imm64(mut ptr: *mut u32, rd: u32, val: u64) -> usize {
    write_word(ptr, movz(rd, val as u16, 0));
    let mut n = 1;
    for shift in [16u32, 32, 48] {
        let part = ((val >> shift) & 0xffff) as u16;
        if part != 0 {
            ptr = unsafe { ptr.add(1) };
            write_word(ptr, movk(rd, part, shift));
            n += 1;
        }
    }
    n
}

/// ADR-094 (G4): a syscall pointer is either read (the kernel copies *from*
/// EL0) or write (the kernel copies *to* EL0). The confused-deputy fix is to
/// require the matching **EL0** permission on every page, not just that the
/// page is mapped in the user TTBR0: a read needs EL0-readable (AP[1]=1), a
/// write needs EL0-writable (AP[1]=1, AP[2]=0). Execute-only (AP[1]=0) and
/// kernel-only pages fail, so EL0 can no longer make `copy_user` /
/// `copy_to_user` touch the `_start` stub, kernel text/data, a kernel-only
/// map-window page, or EL0's own execute-only text.
#[derive(Clone, Copy, PartialEq, Eq)]
enum UAccess {
    Read,
    Write,
}

fn user_range_ok_max(ptr: u64, len: u64, max: u64, access: UAccess) -> bool {
    if len == 0 || len > max {
        return false;
    }
    if ptr.checked_add(len).is_none() {
        return false;
    }
    let last = ptr + len - 1;
    // Walks mask to 39 bits. A TTBR1 / non-canonical alias of a mapped
    // page would otherwise pass, and `copy_user` would load the original
    // pointer (kernel abort instead of `uart_write` → 0).
    if ptr != paging::identity_pa(ptr) || last != paging::identity_pa(last) {
        return false;
    }
    let mut page = ptr & !0xfff;
    let end = last & !0xfff;
    while page <= end {
        if !paging::user_mapped(page) || !paging::is_mapped(page) {
            return false;
        }
        if !page_el0_access_ok(page, access) {
            return false;
        }
        if page == end {
            break;
        }
        page = match page.checked_add(4096) {
            Some(p) => p,
            None => return false,
        };
    }
    true
}

/// ADR-094 (G4): does EL0 itself have the access it is asking the kernel to
/// perform on `page`? The `sysptr-leak-probe` build skips this check (keeping
/// only the mapping/identity guards) so smoke can prove the check is what
/// fails EL0 closed.
#[cfg(not(feature = "sysptr-leak-probe"))]
fn page_el0_access_ok(page: u64, access: UAccess) -> bool {
    let ok = match paging::user_el0_access(page) {
        Some((readable, writable)) => match access {
            UAccess::Read => readable,
            UAccess::Write => writable,
        },
        None => false,
    };
    if !ok {
        SYSPTR_DENIED.fetch_add(1, Ordering::SeqCst);
    }
    ok
}

/// ADR-094: refusals made by the EL0-permission check itself (not by the
/// mapping / length / identity guards). Probes require an exact delta of 1
/// so each refusal is attributable to this check. Stays 0 in the
/// `sysptr-leak-probe` build.
static SYSPTR_DENIED: AtomicU64 = AtomicU64::new(0);

/// Negative build: pretend EL0 always has permission (confused deputy).
#[cfg(feature = "sysptr-leak-probe")]
fn page_el0_access_ok(_page: u64, _access: UAccess) -> bool {
    true
}

fn copy_user(ptr: u64, len: usize, dst: &mut [u8]) -> Option<usize> {
    copy_user_max(ptr, len, dst, UART_WRITE_MAX)
}

fn copy_user_max(ptr: u64, len: usize, dst: &mut [u8], max: u64) -> Option<usize> {
    if len > dst.len() || !user_range_ok_max(ptr, len as u64, max, UAccess::Read) {
        return None;
    }
    // ADR-080: PSTATE.PAN blocks privileged loads of EL0-accessible pages.
    crate::pan::with_user_access(|| {
        for i in 0..len {
            dst[i] = unsafe { core::ptr::read_volatile((ptr as *const u8).add(i)) };
        }
        Some(len)
    })
}

fn copy_to_user(ptr: u64, src: &[u8]) -> Option<usize> {
    if src.is_empty() {
        return Some(0);
    }
    if !user_range_ok_max(ptr, src.len() as u64, FS_IO_MAX, UAccess::Write) {
        return None;
    }
    crate::pan::with_user_access(|| {
        for (i, &b) in src.iter().enumerate() {
            unsafe {
                core::ptr::write_volatile((ptr as *mut u8).add(i), b);
            }
        }
        Some(src.len())
    })
}

fn path_from_user(ptr: u64, len: u64) -> Option<[u8; vfs::PATH_MAX]> {
    if len == 0 || len > vfs::PATH_MAX as u64 {
        return None;
    }
    let mut buf = [0u8; vfs::PATH_MAX];
    copy_user_max(ptr, len as usize, &mut buf, vfs::PATH_MAX as u64)?;
    Some(buf)
}

fn path_str(buf: &[u8], len: u64) -> Option<&str> {
    let n = len as usize;
    if n > buf.len() {
        return None;
    }
    let s = core::str::from_utf8(&buf[..n]).ok()?;
    if vfs::valid_path(s) {
        Some(s)
    } else {
        None
    }
}

fn mov_reg(rd: u32, rn: u32) -> u32 {
    // `MOV Xd, Xn` == `ORR Xd, XZR, Xn`
    0xAA0003E0 | (rn << 16) | rd
}

/// Dispatch a public ABI `SVC`. Probe immediates 0–2 stay in `exception`.
pub fn dispatch(ctx: &mut ExceptionContext) -> Option<SvcAction> {
    let nr = ctx.esr & 0xffff;
    let r = dispatch_inner(ctx, nr);
    if r.is_some() && nr != SYS_EXIT {
        LAST_SVC_RET.store(ctx.x[0], Ordering::SeqCst);
    }
    r
}

fn dispatch_inner(ctx: &mut ExceptionContext, nr: u64) -> Option<SvcAction> {
    match nr {
        SYS_YIELD => {
            el0::note_active_while_standing();
            YIELD_COUNT.fetch_add(1, Ordering::SeqCst);
            YIELD_OK.store(true, Ordering::SeqCst);
            ctx.x[0] = 0;
            uart::write_str_raw("svc: yield\n");
            // ADR-092 (G1): one live EL0-reachability walk while the app stands.
            crate::reach::on_yield();
            Some(SvcAction::StayEl0)
        }
        SYS_UART_WRITE => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let mut buf = [0u8; UART_WRITE_MAX as usize];
            match copy_user(ptr, len as usize, &mut buf) {
                Some(n) => {
                    uart::write_bytes_raw(&buf[..n]);
                    UART_LAST.store(n as u64, Ordering::SeqCst);
                    UART_OK.store(true, Ordering::SeqCst);
                    ctx.x[0] = n as u64;
                    let mut w = uart::raw();
                    let _ = writeln!(w, "svc: uart n={n}");
                    Some(SvcAction::StayEl0)
                }
                None => {
                    UART_LAST.store(0, Ordering::SeqCst);
                    ctx.x[0] = 0;
                    Some(SvcAction::StayEl0)
                }
            }
        }
        SYS_EXIT => {
            let status = ctx.x[0];
            EXIT_STATUS.store(status, Ordering::SeqCst);
            EXIT_OK.store(true, Ordering::SeqCst);
            uart::write_str_raw("svc: exit\n");
            Some(SvcAction::ReturnEl1)
        }
        SYS_FS_CREATE => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let fd = path_from_user(ptr, len)
                .and_then(|buf| path_str(&buf, len).and_then(|p| vfs::create(p).ok()))
                .unwrap_or(0);
            FS_CREATE.store(fd as u64, Ordering::SeqCst);
            ctx.x[0] = fd as u64;
            Some(SvcAction::StayEl0)
        }
        SYS_FS_OPEN => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let fd = path_from_user(ptr, len)
                .and_then(|buf| path_str(&buf, len).and_then(|p| vfs::open(p).ok()))
                .unwrap_or(0);
            FS_OPEN.store(fd as u64, Ordering::SeqCst);
            ctx.x[0] = fd as u64;
            Some(SvcAction::StayEl0)
        }
        SYS_FS_READ => {
            el0::note_active_while_standing();
            let fd = ctx.x[0] as u32;
            let ptr = ctx.x[1];
            let len = ctx.x[2];
            if len == 0 || len > FS_IO_MAX || !user_range_ok_max(ptr, len, FS_IO_MAX, UAccess::Write) {
                FS_READ.store(FS_ERR, Ordering::SeqCst);
                ctx.x[0] = FS_ERR;
                return Some(SvcAction::StayEl0);
            }
            let mut tmp = [0u8; FS_IO_MAX as usize];
            match vfs::read(fd, &mut tmp[..len as usize]) {
                Ok(n) => match copy_to_user(ptr, &tmp[..n]) {
                    Some(_) => {
                        if n > 0 {
                            vfs::note_el0_read(&tmp[..n]);
                        }
                        FS_READ.store(n as u64, Ordering::SeqCst);
                        ctx.x[0] = n as u64;
                    }
                    None => {
                        FS_READ.store(FS_ERR, Ordering::SeqCst);
                        ctx.x[0] = FS_ERR;
                    }
                },
                Err(_) => {
                    FS_READ.store(FS_ERR, Ordering::SeqCst);
                    ctx.x[0] = FS_ERR;
                }
            }
            Some(SvcAction::StayEl0)
        }
        SYS_FS_WRITE => {
            el0::note_active_while_standing();
            let fd = ctx.x[0] as u32;
            let ptr = ctx.x[1];
            let len = ctx.x[2];
            let mut tmp = [0u8; FS_IO_MAX as usize];
            let n = match copy_user_max(ptr, len as usize, &mut tmp, FS_IO_MAX)
                .and_then(|n| vfs::write(fd, &tmp[..n]).ok())
            {
                Some(n) => n as u64,
                None => 0,
            };
            FS_WRITE.store(n, Ordering::SeqCst);
            ctx.x[0] = n;
            Some(SvcAction::StayEl0)
        }
        SYS_FS_CLOSE => {
            el0::note_active_while_standing();
            match vfs::close(ctx.x[0] as u32) {
                Ok(()) => {
                    FS_CLOSE_OK.store(true, Ordering::SeqCst);
                    ctx.x[0] = 0;
                }
                Err(_) => {
                    ctx.x[0] = FS_ERR;
                }
            }
            Some(SvcAction::StayEl0)
        }
        SYS_NET_MAC => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            if len < NET_MAC_LEN || !user_range_ok_max(ptr, NET_MAC_LEN, NET_MAC_LEN, UAccess::Write) {
                NET_MAC_LAST.store(0, Ordering::SeqCst);
                ctx.x[0] = 0;
                return Some(SvcAction::StayEl0);
            }
            match virtio::guest_mac() {
                Some(mac) => match copy_to_user(ptr, &mac) {
                    Some(_) => {
                        NET_MAC_LAST.store(NET_MAC_LEN, Ordering::SeqCst);
                        ctx.x[0] = NET_MAC_LEN;
                    }
                    None => {
                        NET_MAC_LAST.store(0, Ordering::SeqCst);
                        ctx.x[0] = 0;
                    }
                },
                None => {
                    NET_MAC_LAST.store(0, Ordering::SeqCst);
                    ctx.x[0] = 0;
                }
            }
            Some(SvcAction::StayEl0)
        }
        SYS_NET_PING => {
            el0::note_active_while_standing();
            if virtio::el0_icmp_ping() {
                NET_PING_OK.store(true, Ordering::SeqCst);
                ctx.x[0] = 0;
            } else {
                NET_PING_OK.store(false, Ordering::SeqCst);
                ctx.x[0] = NET_ERR;
            }
            Some(SvcAction::StayEl0)
        }
        SYS_NET_UDP_DNS => {
            el0::note_active_while_standing();
            if virtio::el0_udp_dns() {
                NET_UDP_DNS_OK.store(true, Ordering::SeqCst);
                ctx.x[0] = 0;
            } else {
                NET_UDP_DNS_OK.store(false, Ordering::SeqCst);
                ctx.x[0] = NET_ERR;
            }
            Some(SvcAction::StayEl0)
        }
        SYS_NET_TCP_ECHO => {
            el0::note_active_while_standing();
            if virtio::el0_tcp_echo() {
                NET_TCP_ECHO_OK.store(true, Ordering::SeqCst);
                ctx.x[0] = 0;
            } else {
                NET_TCP_ECHO_OK.store(false, Ordering::SeqCst);
                ctx.x[0] = NET_ERR;
            }
            Some(SvcAction::StayEl0)
        }
        SYS_FS_MKDIR => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let code = match path_from_user(ptr, len) {
                Some(buf) => match path_str(&buf, len) {
                    Some(p) => match vfs::mkdir(p) {
                        Ok(()) => FS_MKDIR_OK,
                        Err(vfs::FsError::Exists) => FS_MKDIR_EXISTS,
                        Err(vfs::FsError::BadPath) => FS_MKDIR_BAD_PATH,
                        Err(_) => FS_ERR,
                    },
                    None => FS_MKDIR_BAD_PATH,
                },
                None => FS_MKDIR_BAD_PATH,
            };
            FS_MKDIR_LAST.store(code, Ordering::SeqCst);
            ctx.x[0] = code;
            Some(SvcAction::StayEl0)
        }
        _ => None,
    }
}

pub(crate) fn reset_probe_flags() {
    YIELD_COUNT.store(0, Ordering::SeqCst);
    EXIT_STATUS.store(u64::MAX, Ordering::SeqCst);
    UART_LAST.store(u64::MAX, Ordering::SeqCst);
    YIELD_OK.store(false, Ordering::SeqCst);
    UART_OK.store(false, Ordering::SeqCst);
    EXIT_OK.store(false, Ordering::SeqCst);
    reset_fs_flags();
}

pub(crate) fn reset_fs_flags() {
    FS_CREATE.store(0, Ordering::SeqCst);
    FS_OPEN.store(0, Ordering::SeqCst);
    FS_READ.store(u64::MAX, Ordering::SeqCst);
    FS_WRITE.store(u64::MAX, Ordering::SeqCst);
    FS_CLOSE_OK.store(false, Ordering::SeqCst);
    NET_MAC_LAST.store(u64::MAX, Ordering::SeqCst);
    NET_PING_OK.store(false, Ordering::SeqCst);
    NET_UDP_DNS_OK.store(false, Ordering::SeqCst);
    FS_MKDIR_LAST.store(u64::MAX, Ordering::SeqCst);
    NET_TCP_ECHO_OK.store(false, Ordering::SeqCst);
}

fn write_bytes(base: *mut u8, off: usize, bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        unsafe {
            core::ptr::write_volatile(base.add(off + i), b);
        }
    }
}

/// EL0: create `/eprobe`, write `memfs-el0`, close, open, read, close, exit.
/// ADR-094: path, bytes and the read buffer live on the EL0-RW data page.
fn write_fs_success_payload(ptr: *mut u32, data: *mut u8) {
    let va = EL0_DATA_VA;
    let path_off = 0usize;
    let data_off = 16usize;
    let buf_off = 32usize;
    write_bytes(data, path_off, EL0_FS_PATH);
    write_bytes(data, data_off, EL0_FS_BYTES);
    let mut p = ptr;
    let n = write_imm64(p, 0, va + path_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, EL0_FS_PATH.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC19_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(3, 0));
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 1, va + data_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(2, EL0_FS_BYTES.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC22_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    write_word(p, SVC23_A64);
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 0, va + path_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, EL0_FS_PATH.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC20_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(3, 0));
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 1, va + buf_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(2, 16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC21_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    write_word(p, SVC23_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

/// EL0: create `/kreject`, `SYS_FS_WRITE` from kernel `.data`, then exit.
fn write_fs_reject_payload(ptr: *mut u32, data: *mut u8, bait: u64) {
    let va = EL0_DATA_VA;
    let path_off = 0usize;
    write_bytes(data, path_off, EL0_FS_REJECT_PATH);
    let mut p = ptr;
    let n = write_imm64(p, 0, va + path_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, EL0_FS_REJECT_PATH.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC19_A64);
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 1, bait);
    p = unsafe { p.add(n) };
    write_word(p, movz(2, 4, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC22_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

pub(crate) fn run_fs_el0_payload() -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    reset_probe_flags();
    with_el0_data(|data| with_el0_page(|ptr, va| {
        write_fs_success_payload(ptr, data);
        let user_sp = va + 4096;
        el0::install_standing(va, user_sp, paging::user_ttbr0());
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if el0::is_active() {
            el0::clear_active();
            return false;
        }
        EXIT_OK.load(Ordering::SeqCst)
    }))
}

#[allow(dead_code)] // `#[test_case]` only.
pub(crate) fn fs_el0_reject_kernel_data() -> bool {
    let bait = paging::identity_pa(core::ptr::addr_of!(crate::el0::KERNEL_DATA_BAIT) as u64);
    if paging::user_mapped(bait) {
        return false;
    }
    vfs::reset();
    reset_probe_flags();
    if !run_payload_data(|ptr, data| write_fs_reject_payload(ptr, data, bait)) {
        return false;
    }
    last_fs_create() >= 1 && last_fs_write() == 0
}

fn with_el0_page<F: FnOnce(*mut u32, u64) -> bool>(f: F) -> bool {
    let Some(pa) = frame::alloc() else {
        return false;
    };
    let va = paging::EL0_PAGE;
    if !paging::map_el0_exec(va, pa) {
        frame::free(pa);
        return false;
    }
    let ok = f(va as *mut u32, va);
    let _ = paging::unmap_page(va);
    frame::free(pa);
    ok
}

/// ADR-094 (G4): EL0 data page for kernel-built syscall payloads. The code
/// page is execute-only (EL0 cannot read or write it), so strings the payload
/// hands to a syscall, and buffers the kernel copies into, must live on a
/// page EL0 itself may read/write. EL0-RW, NX, at `EL0_DATA_VA` (the
/// allowlisted `crt-stack-pan` slot). EL1 fills/reads it via the frame alias
/// (PAN blocks the EL0-accessible VA).
pub(crate) const EL0_DATA_VA: u64 = paging::EL0_PAGE + 4096;

fn with_el0_data<F: FnOnce(*mut u8) -> bool>(f: F) -> bool {
    let Some(pa) = frame::alloc() else {
        return false;
    };
    let alias = paging::frame_cpu_va(pa) as *mut u8;
    unsafe { core::ptr::write_bytes(alias, 0, 4096) };
    if !paging::map_el0_rw(EL0_DATA_VA, pa) {
        frame::free(pa);
        return false;
    }
    let ok = f(alias);
    let _ = paging::unmap_page(EL0_DATA_VA);
    frame::free(pa);
    ok
}

/// `SVC #18` / load user ptr+len / `SVC #17` / `SVC #16` with status 0.
/// ADR-094: the message sits on the EL0-RW data page (EL0 must be able to
/// read what it asks the kernel to print).
fn write_success_payload(ptr: *mut u32, data: *mut u8) {
    let msg_va = EL0_DATA_VA;
    let mut p = ptr;
    write_word(p, SVC18_A64);
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 0, msg_va);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, USER_UART_MSG.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC17_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
    write_bytes(data, 0, USER_UART_MSG);
}

/// `SYS_UART_WRITE` from kernel `.data`, then `SYS_EXIT`.
#[allow(dead_code)] // hello kernel uses the success trip only.
fn write_reject_payload(ptr: *mut u32, bait: u64) {
    let mut p = ptr;
    let n = write_imm64(p, 0, bait);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, 4, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC17_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

/// `run_payload` with the ADR-094 EL0-RW data page mapped alongside.
fn run_payload_data<F: FnOnce(*mut u32, *mut u8)>(write: F) -> bool {
    with_el0_data(|data| run_payload(|ptr, _va| write(ptr, data)))
}

fn run_payload<F: FnOnce(*mut u32, u64)>(write: F) -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    reset_probe_flags();
    with_el0_page(|ptr, va| {
        write(ptr, va);
        let user_sp = va + 4096;
        el0::install_standing(va, user_sp, paging::user_ttbr0());
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if el0::is_active() {
            el0::clear_active();
            return false;
        }
        EXIT_OK.load(Ordering::SeqCst)
    })
}

fn abi_success_trip() -> bool {
    if !run_payload_data(|ptr, data| write_success_payload(ptr, data)) {
        return false;
    }
    YIELD_OK.load(Ordering::SeqCst)
        && UART_OK.load(Ordering::SeqCst)
        && UART_LAST.load(Ordering::SeqCst) == USER_UART_MSG.len() as u64
        && EXIT_STATUS.load(Ordering::SeqCst) == 0
        && YIELD_COUNT.load(Ordering::SeqCst) >= 1
}

#[allow(dead_code)] // `#[test_case]` only.
fn abi_reject_kernel_data() -> bool {
    let bait = paging::identity_pa(core::ptr::addr_of!(crate::el0::KERNEL_DATA_BAIT) as u64);
    if paging::user_mapped(bait) {
        return false;
    }
    if !run_payload(|ptr, _va| write_reject_payload(ptr, bait)) {
        return false;
    }
    UART_LAST.load(Ordering::SeqCst) == 0 && !UART_OK.load(Ordering::SeqCst)
}

/// `SYS_UART_WRITE` via the TTBR1 alias of a user-mapped page, then `SYS_EXIT`.
#[allow(dead_code)] // `#[test_case]` only.
fn abi_reject_high_alias() -> bool {
    if !run_payload(|ptr, va| {
        let msg_off = 64u64;
        let dst = unsafe { (ptr as *mut u8).add(msg_off as usize) };
        for i in 0..4 {
            unsafe {
                core::ptr::write_volatile(dst.add(i), b'X');
            }
        }
        write_reject_payload(ptr, paging::to_high_va(va + msg_off));
    }) {
        return false;
    }
    UART_LAST.load(Ordering::SeqCst) == 0 && !UART_OK.load(Ordering::SeqCst)
}

/// ADR-094 (G4) targets outside the payload code page.
/// Kernel-only map-window page (`map_page`: EL1-RW, EL0-none, shared window
/// so it is present in the user TTBR0).
const SYSPTR_KWIN_VA: u64 = paging::MAP_WINDOW + 11 * 4096;
/// EL0 read-only window page (EL0 may read, not write; EL1 read-only too).
/// Reuses the allowlisted ADR-092 `store-ro` slot, so G1's allowlist does
/// not grow (the kernel-window page above is EL0-none: not EL0-reachable).
const SYSPTR_RO_VA: u64 = paging::EL0_STORE_RO_VA;
/// Sentinel written into the RO page frame (via its alias) to prove no write.
const SYSPTR_SENTINEL: u64 = 0x5153_5054_5254_5241; // "AR TP SQ" tag

const SVC_UART_A64: u32 = 0xD4000001 | ((SYS_UART_WRITE as u32) << 5);
const SVC_NETMAC_A64: u32 = 0xD4000001 | ((SYS_NET_MAC as u32) << 5);

/// EL0 payload: `uart_write(target, len)` then `exit(0)`. The kernel read of
/// `target` must be refused (ADR-094 G4), so nothing is printed.
fn write_uart_read_payload(ptr: *mut u32, target: u64, len: u16) {
    let mut p = ptr;
    let n = write_imm64(p, 0, target);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, len, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC_UART_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

/// EL0 payload: `net_mac(target)` then `exit(0)`. The kernel write to
/// `target` must be refused (ADR-094 G4), so `target` is unchanged.
fn write_netmac_payload(ptr: *mut u32, target: u64) {
    let mut p = ptr;
    let n = write_imm64(p, 0, target);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, NET_MAC_LEN as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC_NETMAC_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

/// ADR-094 (G4): EL0 asks `SYS_UART_WRITE` to read `target` (a kernel-only
/// page). The read must be refused and nothing printed. Returns `true` when
/// denied; prints `el0: sys-ptr denied {name}` or `el0: sys-ptr leaked {name}`.
fn sys_ptr_read_denied(name: &str, target: u64) -> bool {
    let before = SYSPTR_DENIED.load(Ordering::SeqCst);
    if !run_payload(|ptr, _va| write_uart_read_payload(ptr, target, 8)) {
        let mut w = uart::raw();
        let _ = writeln!(w, "el0: sys-ptr bad {} setup", name);
        return false;
    }
    let n = UART_LAST.load(Ordering::SeqCst);
    let printed = UART_OK.load(Ordering::SeqCst);
    let checks = SYSPTR_DENIED.load(Ordering::SeqCst) - before;
    let mut w = uart::raw();
    if n == 0 && !printed && checks == 1 {
        let _ = writeln!(w, "el0: sys-ptr denied {}", name);
        true
    } else {
        let _ = writeln!(w, "el0: sys-ptr leaked {} n={}", name, n);
        false
    }
}

/// ADR-094 (G4): verdict for a refused `SYS_NET_MAC` (copy_to_user) write:
/// the syscall returned 0 and the caller saw the target unchanged.
fn sys_ptr_write_verdict(name: &str, intact: bool, checks: u64) -> bool {
    let n = NET_MAC_LAST.load(Ordering::SeqCst);
    let mut w = uart::raw();
    if n == 0 && intact && checks == 1 {
        let _ = writeln!(w, "el0: sys-ptr denied {}", name);
        true
    } else {
        let _ = writeln!(w, "el0: sys-ptr leaked {} n={} intact={} checks={}", name, n, intact, checks);
        false
    }
}

/// EL0 payload: `x0..x2 = a0..a2; SVC #nr; exit(0)`.
fn write_svc3_payload(ptr: *mut u32, nr: u64, a0: u64, a1: u64, a2: u64) {
    let mut p = ptr;
    for (rd, v) in [(0u32, a0), (1, a1), (2, a2)] {
        let n = write_imm64(p, rd, v);
        p = unsafe { p.add(n) };
    }
    write_word(p, 0xD4000001 | ((nr as u32) << 5));
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

/// Placeholder argument: "EL0's own execute-only text" (code page + 0x800).
const XO_ARG: u64 = u64::MAX - 1;

/// ADR-094 (G4): every syscall that takes a user pointer, pointed at a page
/// EL0 may not access that way. Reads target the `_start` stub page; writes
/// target the payload's own execute-only text. Each call must be refused by
/// the EL0-permission check (exactly one `SYSPTR_DENIED` increment) and
/// return that syscall's error value. One line per syscall, then a summary.
fn sys_ptr_sweep() -> bool {
    let stub = paging::KERNEL_TEXT;
    let cases: [(&str, u64, u64, u64, u64, &str, &str, u64); 7] = [
        ("uart_write", SYS_UART_WRITE, stub, 8, 0, "read", "kernel-stub", 0),
        ("fs_create", SYS_FS_CREATE, stub, 8, 0, "read", "kernel-stub", 0),
        ("fs_open", SYS_FS_OPEN, stub, 8, 0, "read", "kernel-stub", 0),
        ("fs_mkdir", SYS_FS_MKDIR, stub, 8, 0, "read", "kernel-stub", FS_MKDIR_BAD_PATH),
        ("fs_write", SYS_FS_WRITE, 1, stub, 8, "read", "kernel-stub", 0),
        ("fs_read", SYS_FS_READ, 1, XO_ARG, 8, "write", "xo-text", FS_ERR),
        ("net_mac", SYS_NET_MAC, XO_ARG, NET_MAC_LEN, 0, "write", "xo-text", 0),
    ];
    let mut denied = 0usize;
    for &(name, nr, a0, a1, a2, kind, target, err) in cases.iter() {
        let before = SYSPTR_DENIED.load(Ordering::SeqCst);
        LAST_SVC_RET.store(u64::MAX - 7, Ordering::SeqCst);
        let ran = with_el0_page(|ptr, va| {
            let fix = |v: u64| if v == XO_ARG { va + 0x800 } else { v };
            write_svc3_payload(ptr, nr, fix(a0), fix(a1), fix(a2));
            reset_probe_flags();
            let user_sp = va + 4096;
            el0::install_standing(va, user_sp, paging::user_ttbr0());
            unsafe {
                exception::eret_to_el0(black_box(va), 0, user_sp);
            }
            if el0::is_active() {
                el0::clear_active();
                return false;
            }
            EXIT_OK.load(Ordering::SeqCst)
        });
        let checks = SYSPTR_DENIED.load(Ordering::SeqCst) - before;
        let r = LAST_SVC_RET.load(Ordering::SeqCst);
        let mut w = uart::raw();
        if ran && checks == 1 && r == err {
            denied += 1;
            let _ = writeln!(w, "el0: sys-ptr denied sys={} nr={} {} {}", name, nr, kind, target);
        } else {
            let _ = writeln!(w, "el0: sys-ptr leaked sys={} nr={} {} {} checks={} ret={:#x}", name, nr, kind, target, checks, r);
        }
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "el0: sys-ptr sweep syscalls={} denied={}", cases.len(), denied);
    denied == cases.len()
}

/// Kernel-only page right after the EL0 data page (unused window slot 4).
const SYSPTR_NEXT_VA: u64 = EL0_DATA_VA + 4096;

/// ADR-094 (G4): a range that starts EL0-readable and ends kernel-only must
/// be refused: the permission check runs on **every** page in the range.
fn sys_ptr_straddle_denied() -> bool {
    let Some(kpa) = frame::alloc() else {
        return false;
    };
    if !paging::map_page(SYSPTR_NEXT_VA, kpa) {
        frame::free(kpa);
        return false;
    }
    let before = SYSPTR_DENIED.load(Ordering::SeqCst);
    let ran = with_el0_data(|_data| {
        run_payload(|ptr, _va| write_uart_read_payload(ptr, SYSPTR_NEXT_VA - 4, 8))
    });
    let checks = SYSPTR_DENIED.load(Ordering::SeqCst) - before;
    let n = UART_LAST.load(Ordering::SeqCst);
    let printed = UART_OK.load(Ordering::SeqCst);
    let _ = paging::unmap_page(SYSPTR_NEXT_VA);
    frame::free(kpa);
    let mut w = uart::raw();
    if ran && n == 0 && !printed && checks == 1 {
        let _ = writeln!(w, "el0: sys-ptr denied straddle");
        true
    } else {
        let _ = writeln!(w, "el0: sys-ptr leaked straddle n={} checks={}", n, checks);
        false
    }
}

/// ADR-094 (G4): the confused-deputy closure. EL0 cannot make a syscall touch
/// a page EL0 itself may not touch. Four refusals, then `el0: sys-ptr ok`:
///   - `SYS_UART_WRITE` read of the `_start` stub page (EL1-only);
///   - `SYS_UART_WRITE` read of a kernel-only map-window page;
///   - `SYS_NET_MAC` (copy_to_user) write of EL0's own execute-only text;
///   - `SYS_NET_MAC` (copy_to_user) write of an EL0 read-only page.
/// The `sysptr-leak-probe` build skips the permission check; then the first
/// read leaks kernel bytes to UART and the probe fails closed (no `ok`).
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_sys_ptr_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if el0::is_active() {
        return false;
    }
    // 0. Positive control: the same copy_to_user syscall into an EL0-RW page
    //    must succeed (6 MAC bytes land), so a later `n=0` really is the
    //    permission check refusing, not a missing NIC.
    let Some(mac) = virtio::guest_mac() else {
        uart::write_str_raw("el0: sys-ptr bad control no-mac\n");
        return false;
    };
    let mut got = [0u8; 6];
    let ctl = with_el0_data(|data| {
        let ran = run_payload(|ptr, _va| write_netmac_payload(ptr, EL0_DATA_VA));
        for (i, b) in got.iter_mut().enumerate() {
            *b = unsafe { core::ptr::read_volatile(data.add(i)) };
        }
        ran
    });
    let n = NET_MAC_LAST.load(Ordering::SeqCst);
    if !ctl || n != NET_MAC_LEN || got != mac {
        let mut w = uart::raw();
        let _ = writeln!(w, "el0: sys-ptr bad control n={}", n);
        return false;
    }
    uart::write_str_raw("el0: sys-ptr control rw-data n=6\n");
    // Keep going after a leak so the negative build reports every case.
    let mut ok = true;
    // 1. Read the identity `_start` stub page (kernel RX, EL0-none in user TTBR0).
    ok &= sys_ptr_read_denied("kernel-stub", paging::KERNEL_TEXT);
    // 2. Read a kernel-only map-window page (EL1-RW, EL0-none).
    let Some(kpa) = frame::alloc() else {
        return false;
    };
    if !paging::map_page(SYSPTR_KWIN_VA, kpa) {
        frame::free(kpa);
        return false;
    }
    ok &= sys_ptr_read_denied("kernel-window", SYSPTR_KWIN_VA);
    let _ = paging::unmap_page(SYSPTR_KWIN_VA);
    frame::free(kpa);
    // 3. Write EL0's own execute-only text (AP[2:1]=00: EL1-RW, EL0 fetch-only).
    //    Target is a sentinel inside the payload's own code page; read it back
    //    (EL1 may read an EL0-none page by VA) before the page is unmapped.
    let xo_off = 0x800usize;
    let mut xo_intact = false;
    let xo_before = SYSPTR_DENIED.load(Ordering::SeqCst);
    let xo_ok = with_el0_page(|ptr, va| {
        write_netmac_payload(ptr, va + xo_off as u64);
        write_bytes(ptr as *mut u8, xo_off, &SYSPTR_SENTINEL.to_le_bytes());
        reset_probe_flags();
        let user_sp = va + 4096;
        el0::install_standing(va, user_sp, paging::user_ttbr0());
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if el0::is_active() {
            el0::clear_active();
            return false;
        }
        let mut b = [0u8; 8];
        for (i, x) in b.iter_mut().enumerate() {
            *x = unsafe { core::ptr::read_volatile((ptr as *const u8).add(xo_off + i)) };
        }
        xo_intact = u64::from_le_bytes(b) == SYSPTR_SENTINEL;
        EXIT_OK.load(Ordering::SeqCst)
    });
    let xo_checks = SYSPTR_DENIED.load(Ordering::SeqCst) - xo_before;
    ok &= xo_ok && sys_ptr_write_verdict("xo-text", xo_intact, xo_checks);
    if !ok {
        // 4 would make EL1 store to an AP[2]=1 page and take an EL1
        // permission fault (kernel park): only run it once 1-3 were refused.
        uart::write_str_raw("el0: sys-ptr skip write-ro\n");
        return false;
    }
    // 4. Write an EL0 read-only page. Fill a sentinel via the frame alias,
    //    then confirm the store never landed.
    let Some(rpa) = frame::alloc() else {
        return false;
    };
    let alias = paging::frame_cpu_va(rpa) as *mut u64;
    unsafe { core::ptr::write_volatile(alias, SYSPTR_SENTINEL) };
    if !paging::map_el0_ro(SYSPTR_RO_VA, rpa) {
        frame::free(rpa);
        return false;
    }
    let ro_before = SYSPTR_DENIED.load(Ordering::SeqCst);
    let ran = run_payload(|ptr, _va| write_netmac_payload(ptr, SYSPTR_RO_VA));
    let ro_checks = SYSPTR_DENIED.load(Ordering::SeqCst) - ro_before;
    let intact = unsafe { core::ptr::read_volatile(alias) } == SYSPTR_SENTINEL;
    let _ = paging::unmap_page(SYSPTR_RO_VA);
    frame::free(rpa);
    if !ran || !sys_ptr_write_verdict("write-ro", intact, ro_checks) {
        return false;
    }
    // 4b. Range straddle: `uart_write(EL0_DATA_VA + 4092, 8)` starts on the
    //     EL0-RW data page and ends on a kernel-only page right after it.
    //     The second page must be checked too (one refusal, nothing printed).
    if !sys_ptr_straddle_denied() {
        return false;
    }
    // 5. Every pointer-taking syscall, one refusal each by the check itself.
    if !sys_ptr_sweep() {
        return false;
    }
    uart::write_str_raw("el0: sys-ptr ok\n");
    true
}

/// Serial proof: EL0 issues yield, uart_write, exit. Not app hosting.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !abi_success_trip() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "svc: ok");
    true
}

#[allow(dead_code)]
pub fn yield_count() -> u64 {
    YIELD_COUNT.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub fn last_uart_write() -> u64 {
    UART_LAST.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub fn last_exit_status() -> u64 {
    EXIT_STATUS.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn yield_seen() -> bool {
    YIELD_OK.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn uart_seen() -> bool {
    UART_OK.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn exit_seen() -> bool {
    EXIT_OK.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_create() -> u64 {
    FS_CREATE.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_open() -> u64 {
    FS_OPEN.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_read() -> u64 {
    FS_READ.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_write() -> u64 {
    FS_WRITE.load(Ordering::SeqCst)
}

pub(crate) fn fs_close_seen() -> bool {
    FS_CLOSE_OK.load(Ordering::SeqCst)
}

#[cfg(test)]
#[test_case]
fn syscall_numbers_are_documented() {
    assert_eq!(svc_a64(SYS_EXIT), SVC16_A64);
    assert_eq!(svc_a64(SYS_UART_WRITE), SVC17_A64);
    assert_eq!(svc_a64(SYS_YIELD), SVC18_A64);
    assert_eq!(svc_a64(SYS_FS_CREATE), SVC19_A64);
    assert_eq!(svc_a64(SYS_FS_OPEN), SVC20_A64);
    assert_eq!(svc_a64(SYS_FS_READ), SVC21_A64);
    assert_eq!(svc_a64(SYS_FS_WRITE), SVC22_A64);
    assert_eq!(svc_a64(SYS_FS_CLOSE), SVC23_A64);
    assert_eq!(svc_a64(SYS_NET_MAC), SVC24_A64);
    assert_eq!(svc_a64(SYS_NET_PING), SVC25_A64);
    assert_eq!(svc_a64(SYS_NET_UDP_DNS), SVC26_A64);
    assert_eq!(svc_a64(SYS_FS_MKDIR), SVC27_A64);
    assert_eq!(svc_a64(SYS_NET_TCP_ECHO), SVC28_A64);
    assert_ne!(SYS_EXIT, SVC_PROBE_RETURN);
    assert_ne!(SYS_EXIT, SVC_PROBE_STANDING);
    assert_ne!(SYS_EXIT, SVC_PROBE_RESTORE);
}

#[cfg(test)]
#[test_case]
fn el0_svc_abi_yield_uart_exit() {
    assert!(paging::mmu_enabled());
    assert!(
        abi_success_trip(),
        "EL0 must SVC yield, uart_write, then exit"
    );
    assert!(!el0::is_active());
    assert_eq!(last_uart_write(), USER_UART_MSG.len() as u64);
    assert_eq!(last_exit_status(), 0);
}

#[cfg(test)]
#[test_case]
fn el0_uart_write_rejects_kernel_data() {
    assert!(paging::mmu_enabled());
    assert!(
        abi_reject_kernel_data(),
        "SYS_UART_WRITE must reject a kernel .data pointer"
    );
    assert!(!el0::is_active());
}

#[cfg(test)]
#[test_case]
fn el0_uart_write_rejects_high_alias() {
    assert!(paging::mmu_enabled());
    assert!(
        abi_reject_high_alias(),
        "SYS_UART_WRITE must reject a TTBR1 alias of a user-mapped page"
    );
    assert!(!el0::is_active());
}

/// ADR-094 (G4): syscall pointers need EL0 permission. Kernel stub / kernel
/// window reads and xo-text / EL0-RO writes are all refused, target intact.
#[cfg(test)]
#[test_case]
fn sys_ptr_requires_el0_permission() {
    assert!(!el0::is_active(), "must start inactive");
    assert!(observe_sys_ptr_probe(), "a syscall touched a page EL0 may not touch (G4)");
    assert!(!el0::is_active());
}

/// ADR-094: the ABI success trip still prints from an EL0-readable page.
#[cfg(test)]
#[test_case]
fn abi_success_from_el0_data_page() {
    assert!(abi_success_trip(), "uart_write from the EL0-RW data page must print");
}
