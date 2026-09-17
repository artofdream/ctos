//! virtio-mmio block + net on QEMU `virt`.
//!
//! Block: Track A / A7 / ADR-028 — one split virtqueue, sector R/W.
//! Net: Track N / N1–N3 / N5 / ADR-066 / ADR-067 / ADR-070 / ADR-074 — RX+TX queues, ARP
//! then ICMP then one UDP datagram vs QEMU user netdev. N4 / ADR-068 and
//! N3.x / ADR-071 expose tiny EL0 SVCs that call into this module (kernel
//! still owns the NIC). ADR-069 times the quiet EL0 `net_ping` path with
//! CNTPCT (`perf: net-ping`); ADR-072 times EL0 `net_udp_dns` (`perf: udp-dns`);
//! ADR-078 times EL0 `net_tcp_echo` (`perf: tcp-echo`).
//! Scan transports, DMA at identity PAs, poll
//! `used.idx`. Not virtio-pci. Not a virtio IRQ.
//! Not BSD sockets / TCP product / DHCP/DNS product / Wi-Fi.

use core::fmt::Write;
use core::ptr::{addr_of, addr_of_mut};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use spin::Mutex;

use crate::paging;
use crate::timer;
use crate::uart;

const MMIO_BASE: usize = 0x0a00_0000;
const MMIO_STRIDE: usize = 0x200;
const MMIO_COUNT: usize = 32;
const MAGIC: u32 = 0x7472_6976; // "virt"
const VERSION_LEGACY: u32 = 1;
const VERSION_MODERN: u32 = 2;
const DEV_BLK: u32 = 2;

const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_DEV_FEAT: usize = 0x010;
const REG_DEV_FEAT_SEL: usize = 0x014;
const REG_DRV_FEAT: usize = 0x020;
const REG_DRV_FEAT_SEL: usize = 0x024;
const REG_GUEST_PAGE: usize = 0x028; // legacy
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_READY: usize = 0x03c; // modern; legacy QueueAlign
const REG_QUEUE_ALIGN: usize = 0x03c; // legacy
const REG_QUEUE_PFN: usize = 0x040; // legacy
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_STATUS: usize = 0x070;
const REG_DESC_LO: usize = 0x080;
const REG_DESC_HI: usize = 0x084;
const REG_AVAIL_LO: usize = 0x090;
const REG_AVAIL_HI: usize = 0x094;
const REG_USED_LO: usize = 0x0a0;
const REG_USED_HI: usize = 0x0a4;
const REG_CONFIG: usize = 0x100;
const PAGE: usize = 4096;

const STATUS_ACK: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_OK: u32 = 4;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_FAILED: u32 = 128;

const VIRTIO_F_VERSION_1: u32 = 1; // features sel=1, bit 0

const QSZ: usize = 8;
const DESC_NEXT: u16 = 1;
const DESC_WRITE: u16 = 2;
const BLK_T_IN: u32 = 0;
const BLK_T_OUT: u32 = 1;
const BLK_S_OK: u8 = 0;
const SECTOR: usize = 512;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Desc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct Avail {
    flags: u16,
    idx: u16,
    ring: [u16; QSZ],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
struct Used {
    flags: u16,
    idx: u16,
    ring: [UsedElem; QSZ],
}

#[repr(C)]
struct BlkReq {
    type_: u32,
    reserved: u32,
    sector: u64,
}

/// Legacy virtio-mmio wants desc+avail in page 0 and used at the next page.
const AVAIL_BYTES: usize = 4 + 2 * QSZ;
const DESC_BYTES: usize = 16 * QSZ;
const LEGACY_PAD: usize = PAGE - DESC_BYTES - AVAIL_BYTES;
const _: () = assert!(LEGACY_PAD > 0);

#[repr(C, align(4096))]
struct Dma {
    desc: [Desc; QSZ],
    avail: Avail,
    _pad: [u8; LEGACY_PAD],
    used: Used,
    req: BlkReq,
    status: u8,
    data: [u8; SECTOR],
}

impl Dma {
    const ZERO: Self = Self {
        desc: [Desc {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }; QSZ],
        avail: Avail {
            flags: 0,
            idx: 0,
            ring: [0; QSZ],
        },
        _pad: [0; LEGACY_PAD],
        used: Used {
            flags: 0,
            idx: 0,
            ring: [UsedElem { id: 0, len: 0 }; QSZ],
        },
        req: BlkReq {
            type_: 0,
            reserved: 0,
            sector: 0,
        },
        status: 0xFF,
        data: [0; SECTOR],
    };
}

struct BlkDev {
    base: usize,
    capacity: u64,
    ready: bool,
}

impl BlkDev {
    const fn empty() -> Self {
        Self {
            base: 0,
            capacity: 0,
            ready: false,
        }
    }
}

static DEV: Mutex<BlkDev> = Mutex::new(BlkDev::empty());
// Single-core: only touched while DEV is held.
static mut DMA: Dma = Dma::ZERO;

static READY: AtomicBool = AtomicBool::new(false);
static CAP: AtomicU64 = AtomicU64::new(0);
static RW_OK: AtomicBool = AtomicBool::new(false);

unsafe fn mmio_write(base: usize, off: usize, val: u32) {
    core::ptr::write_volatile((base + off) as *mut u32, val);
}

unsafe fn mmio_read(base: usize, off: usize) -> u32 {
    core::ptr::read_volatile((base + off) as *const u32)
}

fn dsb() {
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, nomem));
    }
}

fn cache_sync(va: u64, len: usize) {
    let mut p = va & !63;
    let end = va.wrapping_add(len as u64);
    while p < end {
        unsafe {
            core::arch::asm!("dc civac, {x}", x = in(reg) p);
        }
        p = p.wrapping_add(64);
    }
    dsb();
}

fn dma_pa(va: *const u8) -> u64 {
    paging::identity_pa(va as u64)
}

fn find_blk() -> Option<(usize, u32)> {
    for i in 0..MMIO_COUNT {
        let base = MMIO_BASE + i * MMIO_STRIDE;
        unsafe {
            if mmio_read(base, REG_MAGIC) != MAGIC {
                continue;
            }
            let ver = mmio_read(base, REG_VERSION);
            if ver != VERSION_LEGACY && ver != VERSION_MODERN {
                continue;
            }
            if mmio_read(base, REG_DEVICE_ID) == DEV_BLK {
                return Some((base, ver));
            }
        }
    }
    None
}

fn setup_queue(base: usize, ver: u32) -> bool {
    unsafe {
        let dma = addr_of_mut!(DMA);
        (*dma) = Dma::ZERO;
        let desc_pa = dma_pa(addr_of!((*dma).desc) as *const u8);
        mmio_write(base, REG_QUEUE_SEL, 0);
        let max = mmio_read(base, REG_QUEUE_NUM_MAX);
        if max < QSZ as u32 {
            return false;
        }
        mmio_write(base, REG_QUEUE_NUM, QSZ as u32);
        if ver == VERSION_LEGACY {
            if desc_pa & (PAGE as u64 - 1) != 0 {
                return false;
            }
            mmio_write(base, REG_GUEST_PAGE, PAGE as u32);
            mmio_write(base, REG_QUEUE_ALIGN, PAGE as u32);
            mmio_write(base, REG_QUEUE_PFN, (desc_pa / PAGE as u64) as u32);
            dsb();
            true
        } else {
            let avail_pa = dma_pa(addr_of!((*dma).avail) as *const u8);
            let used_pa = dma_pa(addr_of!((*dma).used) as *const u8);
            mmio_write(base, REG_DESC_LO, desc_pa as u32);
            mmio_write(base, REG_DESC_HI, (desc_pa >> 32) as u32);
            mmio_write(base, REG_AVAIL_LO, avail_pa as u32);
            mmio_write(base, REG_AVAIL_HI, (avail_pa >> 32) as u32);
            mmio_write(base, REG_USED_LO, used_pa as u32);
            mmio_write(base, REG_USED_HI, (used_pa >> 32) as u32);
            dsb();
            mmio_write(base, REG_QUEUE_READY, 1);
            mmio_read(base, REG_QUEUE_READY) == 1
        }
    }
}

fn setup(base: usize, ver: u32) -> Option<u64> {
    unsafe {
        mmio_write(base, REG_STATUS, 0);
        dsb();
        mmio_write(base, REG_STATUS, STATUS_ACK | STATUS_DRIVER);

        if ver == VERSION_MODERN {
            mmio_write(base, REG_DEV_FEAT_SEL, 1);
            let hi = mmio_read(base, REG_DEV_FEAT);
            if hi & VIRTIO_F_VERSION_1 == 0 {
                mmio_write(base, REG_STATUS, STATUS_FAILED);
                return None;
            }
            mmio_write(base, REG_DRV_FEAT_SEL, 0);
            mmio_write(base, REG_DRV_FEAT, 0);
            mmio_write(base, REG_DRV_FEAT_SEL, 1);
            mmio_write(base, REG_DRV_FEAT, VIRTIO_F_VERSION_1);
            mmio_write(
                base,
                REG_STATUS,
                STATUS_ACK | STATUS_DRIVER | STATUS_FEATURES_OK,
            );
            if mmio_read(base, REG_STATUS) & STATUS_FEATURES_OK == 0 {
                mmio_write(base, REG_STATUS, STATUS_FAILED);
                return None;
            }
        } else {
            // Legacy: HostFeatures / GuestFeatures at the same offsets, sel=0.
            mmio_write(base, REG_DEV_FEAT_SEL, 0);
            let _host = mmio_read(base, REG_DEV_FEAT);
            mmio_write(base, REG_DRV_FEAT_SEL, 0);
            mmio_write(base, REG_DRV_FEAT, 0);
        }

        if !setup_queue(base, ver) {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }

        let mut st = STATUS_ACK | STATUS_DRIVER | STATUS_OK;
        if ver == VERSION_MODERN {
            st |= STATUS_FEATURES_OK;
        }
        mmio_write(base, REG_STATUS, st);

        let lo = mmio_read(base, REG_CONFIG) as u64;
        let hi = mmio_read(base, REG_CONFIG + 4) as u64;
        let cap = lo | (hi << 32);
        if cap == 0 {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }
        Some(cap)
    }
}

fn xfer(dev: &BlkDev, write: bool, sector: u64, buf: &mut [u8; SECTOR]) -> bool {
    if !dev.ready || sector >= dev.capacity {
        return false;
    }
    unsafe {
        let dma = addr_of_mut!(DMA);
        let last = (*dma).used.idx;
        (*dma).req.type_ = if write { BLK_T_OUT } else { BLK_T_IN };
        (*dma).req.reserved = 0;
        (*dma).req.sector = sector;
        (*dma).status = 0xFF;
        if write {
            (*dma).data.copy_from_slice(buf);
        } else {
            (*dma).data.fill(0);
        }

        let req_pa = dma_pa(addr_of!((*dma).req) as *const u8);
        let data_pa = dma_pa(addr_of!((*dma).data) as *const u8);
        let st_pa = dma_pa(addr_of!((*dma).status) as *const u8);
        (*dma).desc[0] = Desc {
            addr: req_pa,
            len: 16,
            flags: DESC_NEXT,
            next: 1,
        };
        (*dma).desc[1] = Desc {
            addr: data_pa,
            len: SECTOR as u32,
            flags: DESC_NEXT | if write { 0 } else { DESC_WRITE },
            next: 2,
        };
        (*dma).desc[2] = Desc {
            addr: st_pa,
            len: 1,
            flags: DESC_WRITE,
            next: 0,
        };

        let aidx = (*dma).avail.idx;
        (*dma).avail.ring[(aidx as usize) % QSZ] = 0;
        dsb();
        (*dma).avail.idx = aidx.wrapping_add(1);
        // Whole `Dma`: desc+avail (page 0) and used+req+status+data (page 1+).
        // Page-0-only sync leaves the device a stale request/payload (Bugbot).
        cache_sync(addr_of!(*dma) as u64, core::mem::size_of::<Dma>());
        mmio_write(dev.base, REG_QUEUE_NOTIFY, 0);

        let timeout = timer::cntpct().saturating_add(timer_ticks(2));
        let used_off = core::mem::offset_of!(Dma, used);
        let used_span = core::mem::size_of::<Dma>() - used_off;
        loop {
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            if (*dma).used.idx != last {
                break;
            }
            if timer::cntpct() > timeout {
                return false;
            }
        }
        if (*dma).status != BLK_S_OK {
            return false;
        }
        if !write {
            buf.copy_from_slice(&(*dma).data);
        }
        true
    }
}

fn timer_ticks(secs: u64) -> u64 {
    let mut freq: u64;
    unsafe {
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) freq);
    }
    if freq == 0 {
        freq = 62_500_000;
    }
    freq.saturating_mul(secs)
}

/// Discover + program the first virtio-mmio block device.
pub fn init() {
    READY.store(false, Ordering::SeqCst);
    CAP.store(0, Ordering::SeqCst);
    RW_OK.store(false, Ordering::SeqCst);
    let mut dev = DEV.lock();
    *dev = BlkDev::empty();
    let Some((base, ver)) = find_blk() else {
        return;
    };
    let Some(cap) = setup(base, ver) else {
        return;
    };
    dev.base = base;
    dev.capacity = cap;
    dev.ready = true;
    READY.store(true, Ordering::SeqCst);
    CAP.store(cap, Ordering::SeqCst);
}

pub fn ready() -> bool {
    READY.load(Ordering::SeqCst)
}

pub fn capacity() -> u64 {
    CAP.load(Ordering::SeqCst)
}

pub fn read_sector(sector: u64, buf: &mut [u8; SECTOR]) -> bool {
    let dev = DEV.lock();
    xfer(&dev, false, sector, buf)
}

pub fn write_sector(sector: u64, buf: &[u8; SECTOR]) -> bool {
    let dev = DEV.lock();
    let mut tmp = [0u8; SECTOR];
    tmp.copy_from_slice(buf);
    xfer(&dev, true, sector, &mut tmp)
}

fn sector_rw() -> bool {
    RW_OK.store(false, Ordering::SeqCst);
    let cap = capacity();
    if !ready() || cap < 2 {
        return false;
    }
    let mut boot = [0u8; SECTOR];
    if !read_sector(0, &mut boot) || boot[510] != 0x55 || boot[511] != 0xAA {
        return false;
    }
    let last = cap - 1;
    let mut pattern = [0u8; SECTOR];
    for (i, b) in pattern.iter_mut().enumerate() {
        *b = (0xA0 + (i & 0x0F)) as u8;
    }
    if !write_sector(last, &pattern) {
        return false;
    }
    let mut back = [0u8; SECTOR];
    if !read_sector(last, &mut back) || back != pattern {
        return false;
    }
    let zeros = [0u8; SECTOR];
    let _ = write_sector(last, &zeros);
    RW_OK.store(true, Ordering::SeqCst);
    true
}

/// Serial proof: virtqueue + sector R/W. Host `-drive` alone is not this.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() {
        return false;
    }
    if !ready() {
        return false;
    }
    uart::write_str_raw("blk: virtio\n");
    let mut w = uart::raw();
    let _ = writeln!(w, "blk: cap sectors={}", capacity());
    if !sector_rw() {
        return false;
    }
    uart::write_str_raw("blk: rw\n");
    let mut w = uart::raw();
    let _ = writeln!(w, "blk: ok");
    true
}

#[cfg(test)]
#[test_case]
fn virtio_blk_capacity_and_rw() {
    assert!(ready(), "virtio-blk must be attached (qemu -drive)");
    assert!(capacity() >= 2);
    assert!(sector_rw());
    assert!(RW_OK.load(Ordering::SeqCst));
}

#[cfg(test)]
#[test_case]
fn virtio_blk_boot_sector_is_fat() {
    assert!(ready());
    let mut boot = [0u8; SECTOR];
    assert!(read_sector(0, &mut boot));
    assert_eq!(&boot[510..512], &[0x55, 0xAA]);
    assert_eq!(&boot[54..62], b"FAT16   ");
}

// ---------------------------------------------------------------------------
// virtio-net (Track N / N1–N3 / N5 / ADR-066 / ADR-067 / ADR-070 / ADR-074) —
// discover + ARP + ICMP + minimal UDP + thin TCP vs QEMU user netdev.
// Not a BSD sockets product. Not listen/accept. Not DHCP/DNS as product. Not Wi-Fi.
// ---------------------------------------------------------------------------

const DEV_NET: u32 = 1;
const VIRTIO_NET_F_MAC: u32 = 1 << 5;
const NET_HDR: usize = 10; // no MRG_RXBUF → no num_buffers
const NET_FRAME: usize = 256; // ARP/ICMP/UDP DNS reply; keep DMA modest
const NET_Q_RX: u32 = 0;
const NET_Q_TX: u32 = 1;

/// Legacy page layout + virtio_net_hdr + small Ethernet buffer.
#[repr(C, align(4096))]
struct NetDma {
    desc: [Desc; QSZ],
    avail: Avail,
    _pad: [u8; LEGACY_PAD],
    used: Used,
    hdr: [u8; NET_HDR],
    frame: [u8; NET_FRAME],
}

impl NetDma {
    const ZERO: Self = Self {
        desc: [Desc {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }; QSZ],
        avail: Avail {
            flags: 0,
            idx: 0,
            ring: [0; QSZ],
        },
        _pad: [0; LEGACY_PAD],
        used: Used {
            flags: 0,
            idx: 0,
            ring: [UsedElem { id: 0, len: 0 }; QSZ],
        },
        hdr: [0; NET_HDR],
        frame: [0; NET_FRAME],
    };
}

struct NetDev {
    base: usize,
    ver: u32,
    mac: [u8; 6],
    ready: bool,
}

impl NetDev {
    const fn empty() -> Self {
        Self {
            base: 0,
            ver: 0,
            mac: [0; 6],
            ready: false,
        }
    }
}

static NET: Mutex<NetDev> = Mutex::new(NetDev::empty());
static mut NET_RX: NetDma = NetDma::ZERO;
static mut NET_TX: NetDma = NetDma::ZERO;
static NET_READY: AtomicBool = AtomicBool::new(false);
static NET_TX_OK: AtomicBool = AtomicBool::new(false);
static NET_RX_OK: AtomicBool = AtomicBool::new(false);
static NET_ICMP_TX_OK: AtomicBool = AtomicBool::new(false);
static NET_ICMP_RX_OK: AtomicBool = AtomicBool::new(false);
static NET_PING_OK: AtomicBool = AtomicBool::new(false);
static NET_UDP_TX_OK: AtomicBool = AtomicBool::new(false);
static NET_UDP_RX_OK: AtomicBool = AtomicBool::new(false);
static NET_UDP_OK: AtomicBool = AtomicBool::new(false);
static NET_TCP_SYN_OK: AtomicBool = AtomicBool::new(false);
static NET_TCP_EST_OK: AtomicBool = AtomicBool::new(false);
static NET_TCP_TX_OK: AtomicBool = AtomicBool::new(false);
static NET_TCP_RX_OK: AtomicBool = AtomicBool::new(false);
static NET_TCP_OK: AtomicBool = AtomicBool::new(false);
/// ADR-069: last EL0 `net_ping` quiet ARP+ICMP CNTPCT sample (lab only).
static NET_PING_TICKS: AtomicU64 = AtomicU64::new(0);
/// ADR-072: last EL0 `net_udp_dns` quiet ARP+UDP DNS CNTPCT sample (lab only).
static NET_UDP_DNS_TICKS: AtomicU64 = AtomicU64::new(0);
/// ADR-078: last EL0 `net_tcp_echo` quiet ARP+TCP guestfwd CNTPCT sample (lab only).
static NET_TCP_ECHO_TICKS: AtomicU64 = AtomicU64::new(0);

fn find_net() -> Option<(usize, u32)> {
    for i in 0..MMIO_COUNT {
        let base = MMIO_BASE + i * MMIO_STRIDE;
        unsafe {
            if mmio_read(base, REG_MAGIC) != MAGIC {
                continue;
            }
            let ver = mmio_read(base, REG_VERSION);
            if ver != VERSION_LEGACY && ver != VERSION_MODERN {
                continue;
            }
            if mmio_read(base, REG_DEVICE_ID) == DEV_NET {
                return Some((base, ver));
            }
        }
    }
    None
}

fn setup_net_queue(base: usize, ver: u32, qidx: u32, dma: *mut NetDma) -> bool {
    unsafe {
        *dma = NetDma::ZERO;
        let desc_pa = dma_pa(addr_of!((*dma).desc) as *const u8);
        mmio_write(base, REG_QUEUE_SEL, qidx);
        let max = mmio_read(base, REG_QUEUE_NUM_MAX);
        if max < QSZ as u32 {
            return false;
        }
        mmio_write(base, REG_QUEUE_NUM, QSZ as u32);
        if ver == VERSION_LEGACY {
            if desc_pa & (PAGE as u64 - 1) != 0 {
                return false;
            }
            mmio_write(base, REG_GUEST_PAGE, PAGE as u32);
            mmio_write(base, REG_QUEUE_ALIGN, PAGE as u32);
            mmio_write(base, REG_QUEUE_PFN, (desc_pa / PAGE as u64) as u32);
            dsb();
            true
        } else {
            let avail_pa = dma_pa(addr_of!((*dma).avail) as *const u8);
            let used_pa = dma_pa(addr_of!((*dma).used) as *const u8);
            mmio_write(base, REG_DESC_LO, desc_pa as u32);
            mmio_write(base, REG_DESC_HI, (desc_pa >> 32) as u32);
            mmio_write(base, REG_AVAIL_LO, avail_pa as u32);
            mmio_write(base, REG_AVAIL_HI, (avail_pa >> 32) as u32);
            mmio_write(base, REG_USED_LO, used_pa as u32);
            mmio_write(base, REG_USED_HI, (used_pa >> 32) as u32);
            dsb();
            mmio_write(base, REG_QUEUE_READY, 1);
            mmio_read(base, REG_QUEUE_READY) == 1
        }
    }
}

fn read_mac(base: usize) -> [u8; 6] {
    let mut mac = [0u8; 6];
    unsafe {
        for i in 0..6 {
            mac[i] = core::ptr::read_volatile((base + REG_CONFIG + i) as *const u8);
        }
    }
    mac
}

fn setup_net(base: usize, ver: u32) -> Option<[u8; 6]> {
    unsafe {
        mmio_write(base, REG_STATUS, 0);
        dsb();
        mmio_write(base, REG_STATUS, STATUS_ACK | STATUS_DRIVER);

        if ver == VERSION_MODERN {
            mmio_write(base, REG_DEV_FEAT_SEL, 1);
            let hi = mmio_read(base, REG_DEV_FEAT);
            if hi & VIRTIO_F_VERSION_1 == 0 {
                mmio_write(base, REG_STATUS, STATUS_FAILED);
                return None;
            }
            mmio_write(base, REG_DEV_FEAT_SEL, 0);
            let lo = mmio_read(base, REG_DEV_FEAT);
            let want_lo = lo & VIRTIO_NET_F_MAC;
            mmio_write(base, REG_DRV_FEAT_SEL, 0);
            mmio_write(base, REG_DRV_FEAT, want_lo);
            mmio_write(base, REG_DRV_FEAT_SEL, 1);
            mmio_write(base, REG_DRV_FEAT, VIRTIO_F_VERSION_1);
            mmio_write(
                base,
                REG_STATUS,
                STATUS_ACK | STATUS_DRIVER | STATUS_FEATURES_OK,
            );
            if mmio_read(base, REG_STATUS) & STATUS_FEATURES_OK == 0 {
                mmio_write(base, REG_STATUS, STATUS_FAILED);
                return None;
            }
        } else {
            mmio_write(base, REG_DEV_FEAT_SEL, 0);
            let host = mmio_read(base, REG_DEV_FEAT);
            mmio_write(base, REG_DRV_FEAT_SEL, 0);
            mmio_write(base, REG_DRV_FEAT, host & VIRTIO_NET_F_MAC);
        }

        if !setup_net_queue(base, ver, NET_Q_RX, addr_of_mut!(NET_RX)) {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }
        if !setup_net_queue(base, ver, NET_Q_TX, addr_of_mut!(NET_TX)) {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }

        let mut st = STATUS_ACK | STATUS_DRIVER | STATUS_OK;
        if ver == VERSION_MODERN {
            st |= STATUS_FEATURES_OK;
        }
        mmio_write(base, REG_STATUS, st);

        let mac = read_mac(base);
        // Reject an all-zero MAC (config unread / device not net).
        if mac.iter().all(|&b| b == 0) {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }
        Some(mac)
    }
}

fn net_wait_used(dma: *mut NetDma, last: u16, secs_num: u64, secs_den: u64) -> bool {
    unsafe {
        let timeout = timer::cntpct().saturating_add(timer_ticks(secs_num) / secs_den.max(1));
        let used_off = core::mem::offset_of!(NetDma, used);
        let used_span = core::mem::size_of::<NetDma>() - used_off;
        loop {
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            if (*dma).used.idx != last {
                return true;
            }
            if timer::cntpct() > timeout {
                return false;
            }
        }
    }
}

fn net_post_rx(dev: &NetDev) -> Option<u16> {
    unsafe {
        let dma = addr_of_mut!(NET_RX);
        let last = (*dma).used.idx;
        (*dma).hdr.fill(0);
        (*dma).frame.fill(0);
        let hdr_pa = dma_pa(addr_of!((*dma).hdr) as *const u8);
        // One WRITE buffer covering hdr+frame for the device to fill.
        (*dma).desc[0] = Desc {
            addr: hdr_pa,
            len: (NET_HDR + NET_FRAME) as u32,
            flags: DESC_WRITE,
            next: 0,
        };
        let aidx = (*dma).avail.idx;
        (*dma).avail.ring[(aidx as usize) % QSZ] = 0;
        dsb();
        (*dma).avail.idx = aidx.wrapping_add(1);
        cache_sync(addr_of!(*dma) as u64, core::mem::size_of::<NetDma>());
        mmio_write(dev.base, REG_QUEUE_NOTIFY, NET_Q_RX);
        Some(last)
    }
}

/// Build a broadcast ARP request: who-has 10.0.2.2 tell 10.0.2.15.
fn fill_arp_request(mac: &[u8; 6], frame: &mut [u8; NET_FRAME]) -> usize {
    frame.fill(0);
    // Ethernet header
    for i in 0..6 {
        frame[i] = 0xff; // broadcast
        frame[6 + i] = mac[i];
    }
    frame[12] = 0x08;
    frame[13] = 0x06; // ARP
    // ARP
    frame[14] = 0x00;
    frame[15] = 0x01; // htype Ethernet
    frame[16] = 0x08;
    frame[17] = 0x00; // ptype IPv4
    frame[18] = 6; // hlen
    frame[19] = 4; // plen
    frame[20] = 0x00;
    frame[21] = 0x01; // oper request
    for i in 0..6 {
        frame[22 + i] = mac[i]; // sha
    }
    // spa 10.0.2.15
    frame[28] = 10;
    frame[29] = 0;
    frame[30] = 2;
    frame[31] = 15;
    // tha unknown
    // tpa 10.0.2.2 (QEMU SLIRP gateway — answers ARP locally; no external net)
    frame[38] = 10;
    frame[39] = 0;
    frame[40] = 2;
    frame[41] = 2;
    42
}

fn net_tx_frame(dev: &NetDev, frame_len: usize) -> bool {
    if frame_len == 0 || frame_len > NET_FRAME {
        return false;
    }
    unsafe {
        let dma = addr_of_mut!(NET_TX);
        let last = (*dma).used.idx;
        (*dma).hdr.fill(0);
        let hdr_pa = dma_pa(addr_of!((*dma).hdr) as *const u8);
        let frame_pa = dma_pa(addr_of!((*dma).frame) as *const u8);
        (*dma).desc[0] = Desc {
            addr: hdr_pa,
            len: NET_HDR as u32,
            flags: DESC_NEXT,
            next: 1,
        };
        (*dma).desc[1] = Desc {
            addr: frame_pa,
            len: frame_len as u32,
            flags: 0,
            next: 0,
        };
        let aidx = (*dma).avail.idx;
        (*dma).avail.ring[(aidx as usize) % QSZ] = 0;
        dsb();
        (*dma).avail.idx = aidx.wrapping_add(1);
        cache_sync(addr_of!(*dma) as u64, core::mem::size_of::<NetDma>());
        mmio_write(dev.base, REG_QUEUE_NOTIFY, NET_Q_TX);
        // 1s budget — SLIRP TX completion is local.
        if !net_wait_used(dma, last, 1, 1) {
            return false;
        }
        true
    }
}

fn is_arp_reply_from_gateway(frame: &[u8], len: usize) -> bool {
    if len < 42 {
        return false;
    }
    // ethertype ARP
    if frame[12] != 0x08 || frame[13] != 0x06 {
        return false;
    }
    // oper reply
    if frame[20] != 0x00 || frame[21] != 0x02 {
        return false;
    }
    // spa 10.0.2.2
    frame[28] == 10 && frame[29] == 0 && frame[30] == 2 && frame[31] == 2
}

/// Ones-complement checksum over `data` (pad odd length with zero).
fn inet_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u32::from(u16::from_be_bytes([data[i], data[i + 1]]));
        i += 2;
    }
    if i < data.len() {
        sum += u32::from(data[i]) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// RX one ARP reply from SLIRP gateway; return gateway MAC (sha) on success.
/// Skip unrelated RX (e.g. TCP FIN after N5) — same pattern as ICMP/UDP waiters.
fn net_rx_arp_reply(dev: &NetDev, mut last: u16) -> Option<[u8; 6]> {
    for _ in 0..8 {
        unsafe {
            let dma = addr_of_mut!(NET_RX);
            // 2s — SLIRP gateway ARP is local and deterministic on GHA.
            if !net_wait_used(dma, last, 2, 1) {
                return None;
            }
            let used_off = core::mem::offset_of!(NetDma, used);
            let used_span = core::mem::size_of::<NetDma>() - used_off;
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            cache_sync(addr_of!((*dma).hdr) as u64, NET_HDR + NET_FRAME);
            let uidx = (*dma).used.idx.wrapping_sub(1) as usize % QSZ;
            let total = (*dma).used.ring[uidx].len as usize;
            if total > NET_HDR {
                let frame_len = total - NET_HDR;
                let n = frame_len.min(NET_FRAME);
                if is_arp_reply_from_gateway(&(*dma).frame, n) {
                    let mut gw = [0u8; 6];
                    for i in 0..6 {
                        gw[i] = (*dma).frame[22 + i];
                    }
                    return Some(gw);
                }
            }
        }
        let Some(new_last) = net_post_rx(dev) else {
            return None;
        };
        last = new_last;
    }
    None
}

const ICMP_ECHO_ID: u16 = 0x6332; // "c2"
const ICMP_ECHO_SEQ: u16 = 1;
const ICMP_PAYLOAD: &[u8] = b"ctos-n2";

/// Build Ethernet+IPv4+ICMP echo request to gateway 10.0.2.2.
fn fill_icmp_echo_request(
    mac: &[u8; 6],
    gw_mac: &[u8; 6],
    frame: &mut [u8; NET_FRAME],
) -> usize {
    frame.fill(0);
    // Ethernet
    frame[0..6].copy_from_slice(gw_mac);
    frame[6..12].copy_from_slice(mac);
    frame[12] = 0x08;
    frame[13] = 0x00; // IPv4

    let icmp_off = 14 + 20;
    let icmp_len = 8 + ICMP_PAYLOAD.len();
    let ip_total = 20 + icmp_len;

    // IPv4 header (checksum filled below)
    frame[14] = 0x45; // ver=4, ihl=5
    frame[15] = 0;
    frame[16] = (ip_total >> 8) as u8;
    frame[17] = (ip_total & 0xff) as u8;
    frame[18] = 0x00; // id
    frame[19] = 0x01;
    frame[20] = 0x00; // flags/frag
    frame[21] = 0x00;
    frame[22] = 64; // ttl
    frame[23] = 1; // ICMP
    // checksum at 24..26 = 0 for now
    // src 10.0.2.15
    frame[26] = 10;
    frame[27] = 0;
    frame[28] = 2;
    frame[29] = 15;
    // dst 10.0.2.2
    frame[30] = 10;
    frame[31] = 0;
    frame[32] = 2;
    frame[33] = 2;
    let ip_csum = inet_checksum(&frame[14..34]);
    frame[24] = (ip_csum >> 8) as u8;
    frame[25] = (ip_csum & 0xff) as u8;

    // ICMP echo request
    frame[icmp_off] = 8; // type echo request
    frame[icmp_off + 1] = 0; // code
    // checksum 0 for now
    frame[icmp_off + 4] = (ICMP_ECHO_ID >> 8) as u8;
    frame[icmp_off + 5] = (ICMP_ECHO_ID & 0xff) as u8;
    frame[icmp_off + 6] = (ICMP_ECHO_SEQ >> 8) as u8;
    frame[icmp_off + 7] = (ICMP_ECHO_SEQ & 0xff) as u8;
    frame[icmp_off + 8..icmp_off + icmp_len].copy_from_slice(ICMP_PAYLOAD);
    let icmp_csum = inet_checksum(&frame[icmp_off..icmp_off + icmp_len]);
    frame[icmp_off + 2] = (icmp_csum >> 8) as u8;
    frame[icmp_off + 3] = (icmp_csum & 0xff) as u8;

    14 + ip_total
}

fn is_icmp_echo_reply(frame: &[u8], len: usize) -> bool {
    if len < 14 + 20 + 8 {
        return false;
    }
    if frame[12] != 0x08 || frame[13] != 0x00 {
        return false; // not IPv4
    }
    let ihl = (frame[14] & 0x0f) as usize * 4;
    if ihl < 20 || len < 14 + ihl + 8 {
        return false;
    }
    if frame[23] != 1 {
        return false; // not ICMP
    }
    // src 10.0.2.2
    if !(frame[26] == 10 && frame[27] == 0 && frame[28] == 2 && frame[29] == 2) {
        return false;
    }
    // dst 10.0.2.15
    if !(frame[30] == 10 && frame[31] == 0 && frame[32] == 2 && frame[33] == 15) {
        return false;
    }
    let icmp = 14 + ihl;
    if frame[icmp] != 0 {
        return false; // type echo reply
    }
    if frame[icmp + 1] != 0 {
        return false;
    }
    let id = u16::from_be_bytes([frame[icmp + 4], frame[icmp + 5]]);
    let seq = u16::from_be_bytes([frame[icmp + 6], frame[icmp + 7]]);
    id == ICMP_ECHO_ID && seq == ICMP_ECHO_SEQ
}

/// Wait for an ICMP echo reply from the SLIRP gateway (may skip unrelated RX).
fn net_rx_icmp_echo_reply(dev: &NetDev, mut last: u16) -> bool {
    // Up to a few RX deliveries — SLIRP is local; allow stray frames.
    for _ in 0..8 {
        unsafe {
            let dma = addr_of_mut!(NET_RX);
            if !net_wait_used(dma, last, 2, 1) {
                return false;
            }
            let used_off = core::mem::offset_of!(NetDma, used);
            let used_span = core::mem::size_of::<NetDma>() - used_off;
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            cache_sync(addr_of!((*dma).hdr) as u64, NET_HDR + NET_FRAME);
            let uidx = (*dma).used.idx.wrapping_sub(1) as usize % QSZ;
            let total = (*dma).used.ring[uidx].len as usize;
            if total > NET_HDR {
                let frame_len = total - NET_HDR;
                let n = frame_len.min(NET_FRAME);
                if is_icmp_echo_reply(&(*dma).frame, n) {
                    return true;
                }
            }
        }
        // Repost RX for the next frame.
        let Some(new_last) = net_post_rx(dev) else {
            return false;
        };
        last = new_last;
    }
    false
}


const UDP_SRC_PORT: u16 = 0x6333; // "c3" ephemeral for N3
const UDP_DNS_DST: u16 = 53;
const UDP_DNS_TXID: u16 = 0x6333;
/// Minimal DNS A query for label "ctos" (not a DNS product — probe only).
const UDP_DNS_QNAME: &[u8] = b"\x04ctos\x00";

/// Build Ethernet+IPv4+UDP+DNS query to SLIRP DNS 10.0.2.3:53.
/// Reuses gateway MAC from N1 ARP (SLIRP L2 for host-side addrs).
fn fill_udp_dns_query(
    mac: &[u8; 6],
    gw_mac: &[u8; 6],
    frame: &mut [u8; NET_FRAME],
) -> usize {
    frame.fill(0);
    frame[0..6].copy_from_slice(gw_mac);
    frame[6..12].copy_from_slice(mac);
    frame[12] = 0x08;
    frame[13] = 0x00; // IPv4

    // DNS message body
    let dns_off = 14 + 20 + 8;
    frame[dns_off] = (UDP_DNS_TXID >> 8) as u8;
    frame[dns_off + 1] = (UDP_DNS_TXID & 0xff) as u8;
    frame[dns_off + 2] = 0x01; // RD
    frame[dns_off + 3] = 0x00;
    frame[dns_off + 4] = 0x00;
    frame[dns_off + 5] = 0x01; // QDCOUNT=1
    // ANCOUNT/NSCOUNT/ARCOUNT = 0
    let mut dns_len = 12;
    frame[dns_off + dns_len..dns_off + dns_len + UDP_DNS_QNAME.len()]
        .copy_from_slice(UDP_DNS_QNAME);
    dns_len += UDP_DNS_QNAME.len();
    frame[dns_off + dns_len] = 0x00;
    frame[dns_off + dns_len + 1] = 0x01; // type A
    frame[dns_off + dns_len + 2] = 0x00;
    frame[dns_off + dns_len + 3] = 0x01; // class IN
    dns_len += 4;

    let udp_len = 8 + dns_len;
    let ip_total = 20 + udp_len;

    // IPv4
    frame[14] = 0x45;
    frame[15] = 0;
    frame[16] = (ip_total >> 8) as u8;
    frame[17] = (ip_total & 0xff) as u8;
    frame[18] = 0x00;
    frame[19] = 0x03; // id
    frame[20] = 0x00;
    frame[21] = 0x00;
    frame[22] = 64; // ttl
    frame[23] = 17; // UDP
    // src 10.0.2.15
    frame[26] = 10;
    frame[27] = 0;
    frame[28] = 2;
    frame[29] = 15;
    // dst 10.0.2.3 (QEMU SLIRP DNS — answers UDP locally; no external net)
    frame[30] = 10;
    frame[31] = 0;
    frame[32] = 2;
    frame[33] = 3;
    let ip_csum = inet_checksum(&frame[14..34]);
    frame[24] = (ip_csum >> 8) as u8;
    frame[25] = (ip_csum & 0xff) as u8;

    // UDP (checksum 0 = optional / unused for IPv4 — honest for this probe)
    let udp_off = 14 + 20;
    frame[udp_off] = (UDP_SRC_PORT >> 8) as u8;
    frame[udp_off + 1] = (UDP_SRC_PORT & 0xff) as u8;
    frame[udp_off + 2] = (UDP_DNS_DST >> 8) as u8;
    frame[udp_off + 3] = (UDP_DNS_DST & 0xff) as u8;
    frame[udp_off + 4] = (udp_len >> 8) as u8;
    frame[udp_off + 5] = (udp_len & 0xff) as u8;
    // checksum left 0

    14 + ip_total
}

fn is_udp_dns_reply(frame: &[u8], len: usize) -> bool {
    if len < 14 + 20 + 8 + 12 {
        return false;
    }
    if frame[12] != 0x08 || frame[13] != 0x00 {
        return false;
    }
    let ihl = (frame[14] & 0x0f) as usize * 4;
    if ihl < 20 || len < 14 + ihl + 8 + 12 {
        return false;
    }
    if frame[23] != 17 {
        return false; // not UDP
    }
    // src 10.0.2.3
    if !(frame[26] == 10 && frame[27] == 0 && frame[28] == 2 && frame[29] == 3) {
        return false;
    }
    // dst 10.0.2.15
    if !(frame[30] == 10 && frame[31] == 0 && frame[32] == 2 && frame[33] == 15) {
        return false;
    }
    let udp = 14 + ihl;
    let sport = u16::from_be_bytes([frame[udp], frame[udp + 1]]);
    let dport = u16::from_be_bytes([frame[udp + 2], frame[udp + 3]]);
    if sport != UDP_DNS_DST || dport != UDP_SRC_PORT {
        return false;
    }
    let dns = udp + 8;
    let txid = u16::from_be_bytes([frame[dns], frame[dns + 1]]);
    // QR bit set (response) + matching transaction id — any RCODE is fine.
    let flags_hi = frame[dns + 2];
    txid == UDP_DNS_TXID && (flags_hi & 0x80) != 0
}

/// Wait for a UDP DNS reply from SLIRP DNS (may skip unrelated RX).
fn net_rx_udp_dns_reply(dev: &NetDev, mut last: u16) -> bool {
    for _ in 0..8 {
        unsafe {
            let dma = addr_of_mut!(NET_RX);
            if !net_wait_used(dma, last, 2, 1) {
                return false;
            }
            let used_off = core::mem::offset_of!(NetDma, used);
            let used_span = core::mem::size_of::<NetDma>() - used_off;
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            cache_sync(addr_of!((*dma).hdr) as u64, NET_HDR + NET_FRAME);
            let uidx = (*dma).used.idx.wrapping_sub(1) as usize % QSZ;
            let total = (*dma).used.ring[uidx].len as usize;
            if total > NET_HDR {
                let frame_len = total - NET_HDR;
                let n = frame_len.min(NET_FRAME);
                if is_udp_dns_reply(&(*dma).frame, n) {
                    return true;
                }
            }
        }
        let Some(new_last) = net_post_rx(dev) else {
            return false;
        };
        last = new_last;
    }
    false
}


// --- N5 / ADR-074: thin TCP active-open + one payload vs guestfwd echo ---
// Target: 10.0.2.4:7 via QEMU `-netdev …,guestfwd=tcp:10.0.2.4:7-cmd:…/tcp-echo-stdio.sh`.
// One connection. No socket table. No listen/accept. Not a BSD sockets product.

const TCP_SRC_PORT: u16 = 0x6334; // ephemeral for N5
const TCP_EL0_SRC_PORT: u16 = 0x6335; // distinct from N5 for EL0 quiet path (ADR-076)
const TCP_DST_PORT: u16 = 7; // guestfwd echo
const TCP_ISN: u32 = 0x1000_0000;
const TCP_EL0_ISN: u32 = 0x2000_0000;
/// Bump so quiet EL0 opens after tcpdemo / prior probes do not reuse the 4-tuple+ISN.
static TCP_EL0_SEQ: AtomicU32 = AtomicU32::new(0);
const TCP_PAYLOAD: &[u8] = b"ctos-tcp\n";
const TCP_FLAG_SYN: u8 = 0x02;
const TCP_FLAG_PSH: u8 = 0x08;
const TCP_FLAG_ACK: u8 = 0x10;

/// TCP checksum over IPv4 pseudo-header + TCP segment (checksum field 0).
fn tcp_checksum(src: [u8; 4], dst: [u8; 4], tcp: &[u8]) -> u16 {
    // 12-byte pseudo + segment; keep modest stack (payload is tiny).
    let mut buf = [0u8; 12 + 20 + 32];
    let need = 12 + tcp.len();
    if need > buf.len() {
        return 0;
    }
    buf[0..4].copy_from_slice(&src);
    buf[4..8].copy_from_slice(&dst);
    buf[8] = 0;
    buf[9] = 6; // TCP
    let tcp_len = tcp.len() as u16;
    buf[10] = (tcp_len >> 8) as u8;
    buf[11] = (tcp_len & 0xff) as u8;
    buf[12..need].copy_from_slice(tcp);
    inet_checksum(&buf[..need])
}

/// Build Ethernet+IPv4+TCP (no options) to 10.0.2.4:7.
fn fill_tcp_segment(
    mac: &[u8; 6],
    gw_mac: &[u8; 6],
    src_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
    frame: &mut [u8; NET_FRAME],
) -> usize {
    frame.fill(0);
    frame[0..6].copy_from_slice(gw_mac);
    frame[6..12].copy_from_slice(mac);
    frame[12] = 0x08;
    frame[13] = 0x00; // IPv4

    let tcp_off = 14 + 20;
    let tcp_len = 20 + payload.len();
    let ip_total = 20 + tcp_len;

    // IPv4
    frame[14] = 0x45;
    frame[15] = 0;
    frame[16] = (ip_total >> 8) as u8;
    frame[17] = (ip_total & 0xff) as u8;
    frame[18] = 0x00;
    frame[19] = 0x04; // id
    frame[20] = 0x00;
    frame[21] = 0x00;
    frame[22] = 64; // ttl
    frame[23] = 6; // TCP
    // src 10.0.2.15
    frame[26] = 10;
    frame[27] = 0;
    frame[28] = 2;
    frame[29] = 15;
    // dst 10.0.2.4 (guestfwd echo — local; no external net)
    frame[30] = 10;
    frame[31] = 0;
    frame[32] = 2;
    frame[33] = 4;
    let ip_csum = inet_checksum(&frame[14..34]);
    frame[24] = (ip_csum >> 8) as u8;
    frame[25] = (ip_csum & 0xff) as u8;

    // TCP header
    frame[tcp_off] = (src_port >> 8) as u8;
    frame[tcp_off + 1] = (src_port & 0xff) as u8;
    frame[tcp_off + 2] = (TCP_DST_PORT >> 8) as u8;
    frame[tcp_off + 3] = (TCP_DST_PORT & 0xff) as u8;
    frame[tcp_off + 4] = (seq >> 24) as u8;
    frame[tcp_off + 5] = (seq >> 16) as u8;
    frame[tcp_off + 6] = (seq >> 8) as u8;
    frame[tcp_off + 7] = (seq & 0xff) as u8;
    frame[tcp_off + 8] = (ack >> 24) as u8;
    frame[tcp_off + 9] = (ack >> 16) as u8;
    frame[tcp_off + 10] = (ack >> 8) as u8;
    frame[tcp_off + 11] = (ack & 0xff) as u8;
    frame[tcp_off + 12] = 0x50; // data offset = 5 (20 bytes), ns=0
    frame[tcp_off + 13] = flags;
    frame[tcp_off + 14] = 0x20; // window 8192
    frame[tcp_off + 15] = 0x00;
    // checksum 0 for now; urg=0
    if !payload.is_empty() {
        frame[tcp_off + 20..tcp_off + 20 + payload.len()].copy_from_slice(payload);
    }
    let src = [10u8, 0, 2, 15];
    let dst = [10u8, 0, 2, 4];
    let csum = tcp_checksum(src, dst, &frame[tcp_off..tcp_off + tcp_len]);
    frame[tcp_off + 16] = (csum >> 8) as u8;
    frame[tcp_off + 17] = (csum & 0xff) as u8;

    14 + ip_total
}

/// Parse a TCP segment from 10.0.2.4:7 → us. Returns (seq, ack, flags, payload_off, payload_len).
fn parse_tcp_from_echo(
    frame: &[u8],
    len: usize,
    local_port: u16,
) -> Option<(u32, u32, u8, usize, usize)> {
    if len < 14 + 20 + 20 {
        return None;
    }
    if frame[12] != 0x08 || frame[13] != 0x00 {
        return None;
    }
    let ihl = (frame[14] & 0x0f) as usize * 4;
    if ihl < 20 || len < 14 + ihl + 20 {
        return None;
    }
    if frame[23] != 6 {
        return None; // not TCP
    }
    // Prefer IPv4 total length over Ethernet frame length (padding can inflate `len`).
    let ip_total = u16::from_be_bytes([frame[16], frame[17]]) as usize;
    if ip_total < ihl + 20 || 14 + ip_total > len {
        return None;
    }
    // src 10.0.2.4
    if !(frame[26] == 10 && frame[27] == 0 && frame[28] == 2 && frame[29] == 4) {
        return None;
    }
    // dst 10.0.2.15
    if !(frame[30] == 10 && frame[31] == 0 && frame[32] == 2 && frame[33] == 15) {
        return None;
    }
    let tcp = 14 + ihl;
    let sport = u16::from_be_bytes([frame[tcp], frame[tcp + 1]]);
    let dport = u16::from_be_bytes([frame[tcp + 2], frame[tcp + 3]]);
    if sport != TCP_DST_PORT || dport != local_port {
        return None;
    }
    let seq = u32::from_be_bytes([
        frame[tcp + 4],
        frame[tcp + 5],
        frame[tcp + 6],
        frame[tcp + 7],
    ]);
    let ack = u32::from_be_bytes([
        frame[tcp + 8],
        frame[tcp + 9],
        frame[tcp + 10],
        frame[tcp + 11],
    ]);
    let data_off = ((frame[tcp + 12] >> 4) as usize) * 4;
    let tcp_seg_len = ip_total - ihl;
    if data_off < 20 || tcp_seg_len < data_off {
        return None;
    }
    let flags = frame[tcp + 13];
    let payload_off = tcp + data_off;
    let payload_len = tcp_seg_len - data_off;
    Some((seq, ack, flags, payload_off, payload_len))
}

fn is_tcp_syn_ack(frame: &[u8], len: usize, local_port: u16, expect_ack: u32) -> Option<u32> {
    let (seq, ack, flags, _, plen) = parse_tcp_from_echo(frame, len, local_port)?;
    if plen != 0 {
        return None;
    }
    if flags & TCP_FLAG_SYN == 0 || flags & TCP_FLAG_ACK == 0 {
        return None;
    }
    if ack != expect_ack {
        return None;
    }
    Some(seq)
}

fn is_tcp_echo_payload(frame: &[u8], len: usize, local_port: u16) -> Option<(u32, u32, usize)> {
    let (seq, ack, flags, off, plen) = parse_tcp_from_echo(frame, len, local_port)?;
    if plen < TCP_PAYLOAD.len() {
        return None;
    }
    // Require ACK (data segments should ack our side); PSH optional.
    if flags & TCP_FLAG_ACK == 0 {
        return None;
    }
    if &frame[off..off + TCP_PAYLOAD.len()] != TCP_PAYLOAD {
        return None;
    }
    Some((seq, ack, TCP_PAYLOAD.len()))
}

/// Wait for SYN-ACK from guestfwd peer; return peer ISN.
fn net_rx_tcp_syn_ack(dev: &NetDev, mut last: u16, local_port: u16, expect_ack: u32) -> Option<u32> {
    for _ in 0..8 {
        unsafe {
            let dma = addr_of_mut!(NET_RX);
            if !net_wait_used(dma, last, 2, 1) {
                return None;
            }
            let used_off = core::mem::offset_of!(NetDma, used);
            let used_span = core::mem::size_of::<NetDma>() - used_off;
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            cache_sync(addr_of!((*dma).hdr) as u64, NET_HDR + NET_FRAME);
            let uidx = (*dma).used.idx.wrapping_sub(1) as usize % QSZ;
            let total = (*dma).used.ring[uidx].len as usize;
            if total > NET_HDR {
                let frame_len = total - NET_HDR;
                let n = frame_len.min(NET_FRAME);
                if let Some(peer_isn) = is_tcp_syn_ack(&(*dma).frame, n, local_port, expect_ack) {
                    return Some(peer_isn);
                }
            }
        }
        let Some(new_last) = net_post_rx(dev) else {
            return None;
        };
        last = new_last;
    }
    None
}

/// Wait for echoed payload from guestfwd peer.
fn net_rx_tcp_echo(dev: &NetDev, mut last: u16, local_port: u16) -> Option<(u32, u32, usize)> {
    for _ in 0..8 {
        unsafe {
            let dma = addr_of_mut!(NET_RX);
            if !net_wait_used(dma, last, 2, 1) {
                return None;
            }
            let used_off = core::mem::offset_of!(NetDma, used);
            let used_span = core::mem::size_of::<NetDma>() - used_off;
            cache_sync(addr_of!((*dma).used) as u64, used_span);
            cache_sync(addr_of!((*dma).hdr) as u64, NET_HDR + NET_FRAME);
            let uidx = (*dma).used.idx.wrapping_sub(1) as usize % QSZ;
            let total = (*dma).used.ring[uidx].len as usize;
            if total > NET_HDR {
                let frame_len = total - NET_HDR;
                let n = frame_len.min(NET_FRAME);
                if let Some(got) = is_tcp_echo_payload(&(*dma).frame, n, local_port) {
                    return Some(got);
                }
            }
        }
        let Some(new_last) = net_post_rx(dev) else {
            return None;
        };
        last = new_last;
    }
    None
}

/// Discover + program the first virtio-mmio net device (RX+TX queues).
pub fn init_net() {
    NET_READY.store(false, Ordering::SeqCst);
    NET_TX_OK.store(false, Ordering::SeqCst);
    NET_RX_OK.store(false, Ordering::SeqCst);
    NET_ICMP_TX_OK.store(false, Ordering::SeqCst);
    NET_ICMP_RX_OK.store(false, Ordering::SeqCst);
    NET_PING_OK.store(false, Ordering::SeqCst);
    NET_UDP_TX_OK.store(false, Ordering::SeqCst);
    NET_UDP_RX_OK.store(false, Ordering::SeqCst);
    NET_UDP_OK.store(false, Ordering::SeqCst);
    NET_TCP_SYN_OK.store(false, Ordering::SeqCst);
    NET_TCP_EST_OK.store(false, Ordering::SeqCst);
    NET_TCP_TX_OK.store(false, Ordering::SeqCst);
    NET_TCP_RX_OK.store(false, Ordering::SeqCst);
    NET_TCP_OK.store(false, Ordering::SeqCst);
    let mut net = NET.lock();
    *net = NetDev::empty();
    let Some((base, ver)) = find_net() else {
        return;
    };
    let Some(mac) = setup_net(base, ver) else {
        return;
    };
    net.base = base;
    net.ver = ver;
    net.mac = mac;
    net.ready = true;
    NET_READY.store(true, Ordering::SeqCst);
}

pub fn net_ready() -> bool {
    NET_READY.load(Ordering::SeqCst)
}

/// Serial proof: discover + ARP TX/RX + ICMP echo + UDP DNS + thin TCP.
/// Host `-netdev` without this guest path is not a probe. Kernel-path.
/// N1/N2: ADR-066/067. N3 UDP: ADR-070. N5 thin TCP: ADR-074 (guestfwd echo).
/// EL0 quiet helpers: ADR-068/069/071.
#[allow(dead_code)]
pub fn observe_net_probe() -> bool {
    if !paging::mmu_enabled() {
        return false;
    }
    if !net_ready() {
        return false;
    }
    let net = NET.lock();
    uart::write_str_raw("net: virtio\n");
    {
        let mut w = uart::raw();
        let _ = writeln!(
            w,
            "net: mac {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            net.mac[0], net.mac[1], net.mac[2], net.mac[3], net.mac[4], net.mac[5]
        );
    }

    let Some(rx_last) = net_post_rx(&net) else {
        return false;
    };

    let frame_len = {
        // SAFETY: NET_TX only touched while NET lock is held (single-core).
        unsafe {
            fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame)
        }
    };

    let t0 = timer::cntpct();
    if !net_tx_frame(&net, frame_len) {
        return false;
    }
    let tx_ticks = timer::cntpct().saturating_sub(t0);
    NET_TX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: tx\n");
    if tx_ticks > 0 {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: net-tx ticks={}", tx_ticks);
    }

    let r0 = timer::cntpct();
    let Some(gw_mac) = net_rx_arp_reply(&net, rx_last) else {
        return false;
    };
    let rx_ticks = timer::cntpct().saturating_sub(r0);
    NET_RX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: rx\n");
    if rx_ticks > 0 {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: net-rx ticks={}", rx_ticks);
    }

    {
        let mut w = uart::raw();
        let _ = writeln!(w, "net: ok");
    }

    // --- N2 / ADR-067: ICMP echo request/reply after ARP ---
    let Some(rx2) = net_post_rx(&net) else {
        return false;
    };
    let icmp_len = unsafe {
        fill_icmp_echo_request(
            &net.mac,
            &gw_mac,
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    let i0 = timer::cntpct();
    if !net_tx_frame(&net, icmp_len) {
        return false;
    }
    let icmp_tx_ticks = timer::cntpct().saturating_sub(i0);
    NET_ICMP_TX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: icmp-tx\n");
    if icmp_tx_ticks > 0 {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: net-icmp-tx ticks={}", icmp_tx_ticks);
    }

    let i1 = timer::cntpct();
    if !net_rx_icmp_echo_reply(&net, rx2) {
        return false;
    }
    let icmp_rx_ticks = timer::cntpct().saturating_sub(i1);
    NET_ICMP_RX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: icmp-rx\n");
    if icmp_rx_ticks > 0 {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: net-icmp-rx ticks={}", icmp_rx_ticks);
    }

    NET_PING_OK.store(true, Ordering::SeqCst);
    {
        let mut w = uart::raw();
        let _ = writeln!(w, "net: ping-ok");
    }

    // --- N3 / ADR-070: minimal UDP TX/RX (DNS query to SLIRP 10.0.2.3:53) ---
    // Not a DNS product — any matching UDP DNS *response* proves transport.
    let Some(rx3) = net_post_rx(&net) else {
        return false;
    };
    let udp_len = unsafe {
        fill_udp_dns_query(
            &net.mac,
            &gw_mac,
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, udp_len) {
        return false;
    }
    NET_UDP_TX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: udp-tx\n");

    if !net_rx_udp_dns_reply(&net, rx3) {
        return false;
    }
    NET_UDP_RX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: udp-rx\n");

    NET_UDP_OK.store(true, Ordering::SeqCst);
    {
        let mut w = uart::raw();
        let _ = writeln!(w, "net: udp-ok");
    }

    // --- N5 / ADR-074: thin TCP active open + one payload vs guestfwd 10.0.2.4:7 ---
    // Not a sockets product — one connection, no listen/accept.
    let Some(rx4) = net_post_rx(&net) else {
        return false;
    };
    let syn_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw_mac,
            TCP_SRC_PORT,
            TCP_ISN,
            0,
            TCP_FLAG_SYN,
            &[],
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, syn_len) {
        return false;
    }
    NET_TCP_SYN_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: tcp-syn\n");

    let Some(peer_isn) = net_rx_tcp_syn_ack(&net, rx4, TCP_SRC_PORT, TCP_ISN.wrapping_add(1)) else {
        return false;
    };

    let Some(rx5) = net_post_rx(&net) else {
        return false;
    };
    let ack_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw_mac,
            TCP_SRC_PORT,
            TCP_ISN.wrapping_add(1),
            peer_isn.wrapping_add(1),
            TCP_FLAG_ACK,
            &[],
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, ack_len) {
        return false;
    }
    NET_TCP_EST_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: tcp-est\n");

    // Payload may arrive on the same RX posting after we TX.
    let data_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw_mac,
            TCP_SRC_PORT,
            TCP_ISN.wrapping_add(1),
            peer_isn.wrapping_add(1),
            TCP_FLAG_PSH | TCP_FLAG_ACK,
            TCP_PAYLOAD,
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, data_len) {
        return false;
    }
    NET_TCP_TX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: tcp-tx\n");

    let Some((_peer_seq, _peer_ack, plen)) = net_rx_tcp_echo(&net, rx5, TCP_SRC_PORT) else {
        return false;
    };
    let _ = plen;
    NET_TCP_RX_OK.store(true, Ordering::SeqCst);
    uart::write_str_raw("net: tcp-rx\n");

    NET_TCP_OK.store(true, Ordering::SeqCst);
    let mut w = uart::raw();
    let _ = writeln!(w, "net: tcp-ok");
    true
}

/// Guest MAC when virtio-net is ready (ADR-068 EL0 `net_mac`).
pub fn guest_mac() -> Option<[u8; 6]> {
    if !net_ready() {
        return None;
    }
    let net = NET.lock();
    Some(net.mac)
}

/// Quiet ARP + ICMP echo vs SLIRP gateway for EL0 `net_ping` (ADR-068).
/// Kernel still programs virtio-net; no serial N1/N2 markers here.
/// ADR-069: samples CNTPCT around the full quiet path and prints
/// `perf: net-ping ticks=<n>` (lab measurement — not a latency SLA).
pub fn el0_icmp_ping() -> bool {
    NET_PING_TICKS.store(0, Ordering::SeqCst);
    if !paging::mmu_enabled() || !net_ready() {
        return false;
    }
    let net = NET.lock();
    let t0 = timer::cntpct();
    let Some(rx_last) = net_post_rx(&net) else {
        return false;
    };
    let arp_len = unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    if !net_tx_frame(&net, arp_len) {
        return false;
    }
    let Some(gw) = net_rx_arp_reply(&net, rx_last) else {
        return false;
    };
    let Some(rx2) = net_post_rx(&net) else {
        return false;
    };
    let icmp_len = unsafe {
        fill_icmp_echo_request(&net.mac, &gw, &mut (*addr_of_mut!(NET_TX)).frame)
    };
    if !net_tx_frame(&net, icmp_len) {
        return false;
    }
    if !net_rx_icmp_echo_reply(&net, rx2) {
        return false;
    }
    let ticks = timer::cntpct().wrapping_sub(t0);
    if ticks == 0 {
        uart::write_str_raw("perf: net-ping missed\n");
        return false;
    }
    NET_PING_TICKS.store(ticks, Ordering::SeqCst);
    {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: net-ping ticks={}", ticks);
    }
    true
}

/// Last ADR-069 `perf: net-ping` sample (0 if none).
#[allow(dead_code)]
pub fn net_ping_ticks() -> u64 {
    NET_PING_TICKS.load(Ordering::SeqCst)
}

/// Quiet ARP + UDP DNS probe vs SLIRP DNS for EL0 `net_udp_dns` (ADR-071).
/// Kernel still programs virtio-net; no serial N3 markers here.
/// DNS is probe bait only — not a guest DNS product.
/// ADR-072: samples CNTPCT around the full quiet path and prints
/// `perf: udp-dns ticks=<n>` (lab measurement — not a latency SLA).
pub fn el0_udp_dns() -> bool {
    NET_UDP_DNS_TICKS.store(0, Ordering::SeqCst);
    if !paging::mmu_enabled() || !net_ready() {
        return false;
    }
    let net = NET.lock();
    let t0 = timer::cntpct();
    let Some(rx_last) = net_post_rx(&net) else {
        return false;
    };
    let arp_len = unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    if !net_tx_frame(&net, arp_len) {
        return false;
    }
    let Some(gw) = net_rx_arp_reply(&net, rx_last) else {
        return false;
    };
    let Some(rx2) = net_post_rx(&net) else {
        return false;
    };
    let udp_len = unsafe {
        fill_udp_dns_query(&net.mac, &gw, &mut (*addr_of_mut!(NET_TX)).frame)
    };
    if !net_tx_frame(&net, udp_len) {
        return false;
    }
    if !net_rx_udp_dns_reply(&net, rx2) {
        return false;
    }
    let ticks = timer::cntpct().wrapping_sub(t0);
    if ticks == 0 {
        uart::write_str_raw("perf: udp-dns missed\n");
        return false;
    }
    NET_UDP_DNS_TICKS.store(ticks, Ordering::SeqCst);
    {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: udp-dns ticks={}", ticks);
    }
    true
}

/// Last ADR-072 `perf: udp-dns` sample (0 if none).
#[allow(dead_code)]
pub fn udp_dns_ticks() -> u64 {
    NET_UDP_DNS_TICKS.load(Ordering::SeqCst)
}

/// Quiet ARP + thin TCP echo vs guestfwd for EL0 `net_tcp_echo` (ADR-076).
/// Kernel still programs virtio-net; no serial N5 markers here.
/// Uses a distinct ephemeral port/ISN from the kernel N5 path so a second
/// open after `observe_net_probe` does not collide.
/// ADR-078: samples CNTPCT around the full quiet path and prints
/// `perf: tcp-echo ticks=<n>` (lab measurement — not a latency SLA).
pub fn el0_tcp_echo() -> bool {
    NET_TCP_ECHO_TICKS.store(0, Ordering::SeqCst);
    if !paging::mmu_enabled() || !net_ready() {
        return false;
    }
    let n = TCP_EL0_SEQ.fetch_add(1, Ordering::SeqCst);
    let src_port = TCP_EL0_SRC_PORT.wrapping_add((n % 16) as u16);
    let isn = TCP_EL0_ISN.wrapping_add(n.wrapping_mul(0x10000));
    let net = NET.lock();
    let t0 = timer::cntpct();
    let Some(rx_last) = net_post_rx(&net) else {
        return false;
    };
    let arp_len = unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    if !net_tx_frame(&net, arp_len) {
        return false;
    }
    let Some(gw) = net_rx_arp_reply(&net, rx_last) else {
        return false;
    };
    let Some(rx2) = net_post_rx(&net) else {
        return false;
    };
    let syn_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw,
            src_port,
            isn,
            0,
            TCP_FLAG_SYN,
            &[],
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, syn_len) {
        return false;
    }
    let Some(peer_isn) =
        net_rx_tcp_syn_ack(&net, rx2, src_port, isn.wrapping_add(1))
    else {
        return false;
    };
    let Some(rx3) = net_post_rx(&net) else {
        return false;
    };
    let ack_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw,
            src_port,
            isn.wrapping_add(1),
            peer_isn.wrapping_add(1),
            TCP_FLAG_ACK,
            &[],
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, ack_len) {
        return false;
    }
    let data_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw,
            src_port,
            isn.wrapping_add(1),
            peer_isn.wrapping_add(1),
            TCP_FLAG_PSH | TCP_FLAG_ACK,
            TCP_PAYLOAD,
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    if !net_tx_frame(&net, data_len) {
        return false;
    }
    let Some((_peer_seq, _peer_ack, plen)) = net_rx_tcp_echo(&net, rx3, src_port) else {
        return false;
    };
    let _ = plen;
    let ticks = timer::cntpct().wrapping_sub(t0);
    if ticks == 0 {
        uart::write_str_raw("perf: tcp-echo missed\n");
        return false;
    }
    NET_TCP_ECHO_TICKS.store(ticks, Ordering::SeqCst);
    {
        let mut w = uart::raw();
        let _ = writeln!(w, "perf: tcp-echo ticks={}", ticks);
    }
    true
}

/// Last ADR-078 `perf: tcp-echo` sample (0 if none).
#[allow(dead_code)]
pub fn tcp_echo_ticks() -> u64 {
    NET_TCP_ECHO_TICKS.load(Ordering::SeqCst)
}


#[cfg(test)]
#[test_case]
fn virtio_net_discover_and_arp() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev)");
    let net = NET.lock();
    assert!(net.mac.iter().any(|&b| b != 0), "MAC from config");
    let Some(rx_last) = net_post_rx(&net) else {
        panic!("post RX buffer");
    };
    let frame_len =
        unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    assert!(net_tx_frame(&net, frame_len), "TX ARP request");
    let gw = net_rx_arp_reply(&net, rx_last)
        .expect("RX ARP reply from QEMU SLIRP gateway 10.0.2.2");
    assert!(gw.iter().any(|&b| b != 0), "gateway MAC from ARP sha");
}

#[cfg(test)]
#[test_case]
fn virtio_net_icmp_echo_ping() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev)");
    let net = NET.lock();
    // ARP first to learn gateway MAC (SLIRP does not need a prior ARP for
    // ICMP, but our TX path addresses Ethernet by gw MAC from ARP sha).
    let Some(rx_last) = net_post_rx(&net) else {
        panic!("post RX for ARP");
    };
    let arp_len =
        unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    assert!(net_tx_frame(&net, arp_len), "TX ARP request");
    let gw = net_rx_arp_reply(&net, rx_last).expect("ARP reply for ICMP");
    let Some(rx2) = net_post_rx(&net) else {
        panic!("post RX for ICMP");
    };
    let icmp_len = unsafe {
        fill_icmp_echo_request(&net.mac, &gw, &mut (*addr_of_mut!(NET_TX)).frame)
    };
    assert!(net_tx_frame(&net, icmp_len), "TX ICMP echo request");
    assert!(
        net_rx_icmp_echo_reply(&net, rx2),
        "RX ICMP echo reply from QEMU SLIRP gateway 10.0.2.2"
    );
}

#[cfg(test)]
#[test_case]
fn el0_net_ping_cntpct_advances() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev)");
    assert!(
        el0_icmp_ping(),
        "quiet ARP+ICMP for EL0 net_ping must succeed"
    );
    let ticks = net_ping_ticks();
    assert!(ticks > 0, "ADR-069 net-ping CNTPCT sample must advance");
}

#[cfg(test)]
#[test_case]
fn virtio_net_udp_dns_probe() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev)");
    let net = NET.lock();
    let Some(rx_last) = net_post_rx(&net) else {
        panic!("post RX for ARP");
    };
    let arp_len =
        unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    assert!(net_tx_frame(&net, arp_len), "TX ARP request");
    let gw = net_rx_arp_reply(&net, rx_last).expect("ARP reply for UDP");
    let Some(rx2) = net_post_rx(&net) else {
        panic!("post RX for UDP");
    };
    let udp_len = unsafe {
        fill_udp_dns_query(&net.mac, &gw, &mut (*addr_of_mut!(NET_TX)).frame)
    };
    assert!(net_tx_frame(&net, udp_len), "TX UDP DNS query to 10.0.2.3:53");
    assert!(
        net_rx_udp_dns_reply(&net, rx2),
        "RX UDP DNS reply from QEMU SLIRP DNS 10.0.2.3"
    );
}

#[cfg(test)]
#[test_case]
fn el0_udp_dns_probe() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev)");
    assert!(
        el0_udp_dns(),
        "quiet ARP+UDP DNS for EL0 net_udp_dns must succeed"
    );
}

#[cfg(test)]
#[test_case]
fn el0_udp_dns_cntpct_advances() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev)");
    assert!(
        el0_udp_dns(),
        "quiet ARP+UDP DNS for EL0 net_udp_dns must succeed"
    );
    let ticks = udp_dns_ticks();
    assert!(ticks > 0, "ADR-072 udp-dns CNTPCT sample must advance");
}

#[cfg(test)]
#[test_case]
fn virtio_net_tcp_echo_probe() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev + guestfwd)");
    let net = NET.lock();
    let Some(rx_last) = net_post_rx(&net) else {
        panic!("post RX for ARP");
    };
    let arp_len =
        unsafe { fill_arp_request(&net.mac, &mut (*addr_of_mut!(NET_TX)).frame) };
    assert!(net_tx_frame(&net, arp_len), "TX ARP request");
    let gw = net_rx_arp_reply(&net, rx_last).expect("ARP reply for TCP");
    let Some(rx2) = net_post_rx(&net) else {
        panic!("post RX for TCP SYN");
    };
    let syn_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw,
            TCP_SRC_PORT,
            TCP_ISN,
            0,
            TCP_FLAG_SYN,
            &[],
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    assert!(net_tx_frame(&net, syn_len), "TX TCP SYN to 10.0.2.4:7");
    let peer_isn = net_rx_tcp_syn_ack(&net, rx2, TCP_SRC_PORT, TCP_ISN.wrapping_add(1))
        .expect("RX TCP SYN-ACK from guestfwd echo");
    let Some(rx3) = net_post_rx(&net) else {
        panic!("post RX after SYN-ACK");
    };
    let ack_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw,
            TCP_SRC_PORT,
            TCP_ISN.wrapping_add(1),
            peer_isn.wrapping_add(1),
            TCP_FLAG_ACK,
            &[],
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    assert!(net_tx_frame(&net, ack_len), "TX TCP ACK");
    let data_len = unsafe {
        fill_tcp_segment(
            &net.mac,
            &gw,
            TCP_SRC_PORT,
            TCP_ISN.wrapping_add(1),
            peer_isn.wrapping_add(1),
            TCP_FLAG_PSH | TCP_FLAG_ACK,
            TCP_PAYLOAD,
            &mut (*addr_of_mut!(NET_TX)).frame,
        )
    };
    assert!(net_tx_frame(&net, data_len), "TX TCP payload");
    let got = net_rx_tcp_echo(&net, rx3, TCP_SRC_PORT).expect("RX TCP echo payload from guestfwd");
    assert_eq!(got.2, TCP_PAYLOAD.len(), "echo length");
}

#[cfg(test)]
#[test_case]
fn el0_tcp_echo_probe() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev + guestfwd)");
    assert!(
        el0_tcp_echo(),
        "quiet ARP+TCP echo for EL0 net_tcp_echo must succeed"
    );
}

#[cfg(test)]
#[test_case]
fn el0_tcp_echo_cntpct_advances() {
    assert!(net_ready(), "virtio-net must be attached (qemu -netdev + guestfwd)");
    assert!(
        el0_tcp_echo(),
        "quiet ARP+TCP echo for EL0 net_tcp_echo must succeed"
    );
    let ticks = tcp_echo_ticks();
    assert!(ticks > 0, "ADR-078 tcp-echo CNTPCT sample must advance");
}
