//! Fourth freestanding sample slot: FAT `/yldemo` → `yield-libctos` (ADR-062).
//!
//! Loads the published ELF beside `/hello`, `/fsdemo`, and `/fatdemo`, maps
//! with the A3 loader, and `ERET`s. The payload issues several cooperative
//! `yield_now()` rounds (deeper than hello's single yield). Not preemption.
//! Not multi-task EL0. Not a process table. Not app-hosting reopen.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fat;
use crate::loader;
use crate::paging;
use crate::syscall;
use crate::uart;

include!(concat!(env!("OUT_DIR"), "/yield_libctos_meta.rs"));

const _: () = assert!(YLDEMO_ELF_LEN > 64);
const _: () = assert!(YLDEMO_LOAD_VA == 0x8000_2000);

const SLOT_PATH: &str = "/yldemo";
const SLOT_MAX: usize = 64 * 1024;
/// Last `uart_write` from `user/yield-libctos` is `libctos: yld-ok\n` (16 bytes).
const YLDEMO_UART_LEN: u64 = 16;
/// Hello uses one yield; this sample must prove more than that.
const YLDEMO_MIN_YIELDS: u64 = 3;

static FAT_OK: AtomicBool = AtomicBool::new(false);
static LOAD_OK: AtomicBool = AtomicBool::new(false);

fn read_slot() -> Option<Vec<u8>> {
    let out = fat::read_file(SLOT_PATH, SLOT_MAX)?;
    if out.len() != YLDEMO_ELF_LEN || out[0] != 0x7f || &out[1..4] != b"ELF" {
        return None;
    }
    Some(out)
}

/// Serial proof: FAT `/yldemo` parsed and `ERET`ed. Keep prior samples intact.
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
    let _ = writeln!(w, "yldemo: fat bytes={}", bytes.len());
    FAT_OK.store(true, Ordering::SeqCst);
    if !loader::run_image_expecting(&bytes, YLDEMO_UART_LEN) {
        return false;
    }
    if syscall::yield_count() < YLDEMO_MIN_YIELDS {
        return false;
    }
    let _ = writeln!(w, "yldemo: mapped segs={} entry={:#x}", img.nseg, img.entry);
    let _ = writeln!(w, "yldemo: ok");
    LOAD_OK.store(true, Ordering::SeqCst);
    true
}

#[cfg(test)]
use crate::vfs;

#[cfg(test)]
#[test_case]
fn yldemo_fat_is_elf() {
    assert!(fat::mounted(), "FAT16 must mount so /yldemo can exist");
    assert!(fat::has_name(SLOT_PATH), "ADR-062 sample slot is FAT /yldemo");
    let bytes = read_slot().expect("VFS /yldemo must be an ELF");
    assert_eq!(&bytes[0..4], b"\x7fELF");
    let img = loader::parse_elf64(&bytes).expect("yldemo ELF must parse");
    assert_eq!(img.entry, paging::EL0_PAGE);
    assert!(img.nseg >= 1);
}

#[cfg(test)]
#[test_case]
fn yldemo_load_from_fat_erets() {
    assert!(paging::mmu_enabled());
    assert!(
        observe_probe(),
        "yldemo slot must load FAT /yldemo, ERET, several yields, and print yldemo: ok"
    );
    assert!(FAT_OK.load(Ordering::SeqCst));
    assert!(LOAD_OK.load(Ordering::SeqCst));
    assert!(syscall::yield_count() >= YLDEMO_MIN_YIELDS);
}

#[cfg(test)]
#[test_case]
fn yldemo_path_is_fat() {
    assert_eq!(SLOT_PATH, "/yldemo");
    assert!(vfs::valid_path(SLOT_PATH));
    assert_eq!(vfs::resolve("/yldemo"), Ok(vfs::Backend::Fat16));
}
