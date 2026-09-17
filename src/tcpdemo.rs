//! Eighth freestanding sample slot: FAT `/tcpdemo` → `tcp-libctos` (ADR-076).
//!
//! Loads the published ELF beside `/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`,
//! `/netdemo`, `/udpdemo`, and `/mkdemo`, maps with the A3 loader, and `ERET`s.
//! The payload exercises EL0 `net_tcp_echo`. Kernel still owns virtio-net.
//! Not BSD sockets. Not listen/accept. Not app-hosting reopen.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fat;
use crate::loader;
use crate::paging;
use crate::uart;

include!(concat!(env!("OUT_DIR"), "/tcp_libctos_meta.rs"));

const _: () = assert!(TCPDEMO_ELF_LEN > 64);
const _: () = assert!(TCPDEMO_LOAD_VA == 0x8000_2000);

const SLOT_PATH: &str = "/tcpdemo";
const SLOT_MAX: usize = 64 * 1024;
/// Last `uart_write` from `user/tcp-libctos` is `libctos: tcp-ok\n` (16 bytes).
const TCPDEMO_UART_LEN: u64 = 16;

static FAT_OK: AtomicBool = AtomicBool::new(false);
static LOAD_OK: AtomicBool = AtomicBool::new(false);

fn read_slot() -> Option<Vec<u8>> {
    let out = fat::read_file(SLOT_PATH, SLOT_MAX)?;
    if out.len() != TCPDEMO_ELF_LEN || out[0] != 0x7f || &out[1..4] != b"ELF" {
        return None;
    }
    Some(out)
}

/// Serial proof: FAT `/tcpdemo` parsed and `ERET`ed. Keep prior samples intact.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    FAT_OK.store(false, Ordering::SeqCst);
    LOAD_OK.store(false, Ordering::SeqCst);
    if !paging::mmu_enabled() || !paging::user_map_ready() || !fat::mounted() {
        return false;
    }
    if !fat::has_name(SLOT_PATH) {
        return false;
    }
    let Some(bytes) = read_slot() else {
        return false;
    };
    let Ok(img) = loader::parse_elf64(&bytes) else {
        return false;
    };
    let mut w = uart::raw();
    let _ = writeln!(w, "tcpdemo: fat bytes={}", bytes.len());
    FAT_OK.store(true, Ordering::SeqCst);
    if !loader::run_image_expecting(&bytes, TCPDEMO_UART_LEN) {
        return false;
    }
    let _ = writeln!(w, "tcpdemo: mapped segs={} entry={:#x}", img.nseg, img.entry);
    let _ = writeln!(w, "tcpdemo: ok");
    LOAD_OK.store(true, Ordering::SeqCst);
    true
}

#[cfg(test)]
use crate::vfs;

#[cfg(test)]
#[test_case]
fn tcpdemo_fat_is_elf() {
    assert!(fat::mounted(), "FAT16 must mount so /tcpdemo can exist");
    assert!(fat::has_name(SLOT_PATH), "ADR-076 sample slot is FAT /tcpdemo");
    let bytes = read_slot().expect("VFS /tcpdemo must be an ELF");
    assert_eq!(&bytes[0..4], b"\x7fELF");
    let img = loader::parse_elf64(&bytes).expect("tcpdemo ELF must parse");
    assert_eq!(img.entry, paging::EL0_PAGE);
    assert!(img.nseg >= 1);
}

#[cfg(test)]
#[test_case]
fn tcpdemo_load_from_fat_erets() {
    assert!(paging::mmu_enabled());
    assert!(
        observe_probe(),
        "tcpdemo slot must load FAT /tcpdemo, ERET, net_tcp_echo, and print tcpdemo: ok"
    );
    assert!(FAT_OK.load(Ordering::SeqCst));
    assert!(LOAD_OK.load(Ordering::SeqCst));
}

#[cfg(test)]
#[test_case]
fn tcpdemo_path_is_fat() {
    assert_eq!(SLOT_PATH, "/tcpdemo");
    assert!(vfs::valid_path(SLOT_PATH));
    assert_eq!(vfs::resolve("/tcpdemo"), Ok(vfs::Backend::Fat16));
}
