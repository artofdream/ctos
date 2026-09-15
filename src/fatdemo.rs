//! Third freestanding sample slot: FAT `/fatdemo` → `fat-libctos` (ADR-061).
//!
//! Loads the published ELF beside `/hello` and `/fsdemo`, maps with the A3
//! loader, and `ERET`s. The payload opens/reads FAT `/probe` (`fat-hi`) via
//! thin VFS (`/` → FAT16). Not POSIX. Not getdents. Not app-hosting reopen.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fat;
use crate::loader;
use crate::paging;
use crate::uart;

include!(concat!(env!("OUT_DIR"), "/fat_libctos_meta.rs"));

const _: () = assert!(FATDEMO_ELF_LEN > 64);
const _: () = assert!(FATDEMO_LOAD_VA == 0x8000_2000);

const SLOT_PATH: &str = "/fatdemo";
const SLOT_MAX: usize = 64 * 1024;
/// Last `uart_write` from `user/fat-libctos` is `libctos: fat-ok\n` (16 bytes).
const FATDEMO_UART_LEN: u64 = 16;

static FAT_OK: AtomicBool = AtomicBool::new(false);
static LOAD_OK: AtomicBool = AtomicBool::new(false);

fn read_slot() -> Option<Vec<u8>> {
    let out = fat::read_file(SLOT_PATH, SLOT_MAX)?;
    if out.len() != FATDEMO_ELF_LEN || out[0] != 0x7f || &out[1..4] != b"ELF" {
        return None;
    }
    Some(out)
}

/// Serial proof: FAT `/fatdemo` parsed and `ERET`ed. Keep `/hello` + `/fsdemo`.
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
    let _ = writeln!(w, "fatdemo: fat bytes={}", bytes.len());
    FAT_OK.store(true, Ordering::SeqCst);
    if !loader::run_image_expecting(&bytes, FATDEMO_UART_LEN) {
        return false;
    }
    let _ = writeln!(w, "fatdemo: mapped segs={} entry={:#x}", img.nseg, img.entry);
    let _ = writeln!(w, "fatdemo: ok");
    LOAD_OK.store(true, Ordering::SeqCst);
    true
}

#[cfg(test)]
use crate::vfs;

#[cfg(test)]
#[test_case]
fn fatdemo_fat_is_elf() {
    assert!(fat::mounted(), "FAT16 must mount so /fatdemo can exist");
    assert!(fat::has_name(SLOT_PATH), "ADR-061 sample slot is FAT /fatdemo");
    let bytes = read_slot().expect("VFS /fatdemo must be an ELF");
    assert_eq!(&bytes[0..4], b"\x7fELF");
    let img = loader::parse_elf64(&bytes).expect("fatdemo ELF must parse");
    assert_eq!(img.entry, paging::EL0_PAGE);
    assert!(img.nseg >= 1);
}

#[cfg(test)]
#[test_case]
fn fatdemo_load_from_fat_erets() {
    assert!(paging::mmu_enabled());
    assert!(
        observe_probe(),
        "fatdemo slot must load FAT /fatdemo, ERET, and print fatdemo: ok"
    );
    assert!(FAT_OK.load(Ordering::SeqCst));
    assert!(LOAD_OK.load(Ordering::SeqCst));
}

#[cfg(test)]
#[test_case]
fn fatdemo_path_is_fat_probe() {
    assert_eq!(SLOT_PATH, "/fatdemo");
    assert!(vfs::valid_path(SLOT_PATH));
    assert!(vfs::valid_path("/probe"));
    assert_eq!(vfs::resolve("/fatdemo"), Ok(vfs::Backend::Fat16));
    assert_eq!(vfs::resolve("/probe"), Ok(vfs::Backend::Fat16));
    assert_ne!(vfs::resolve("/probe"), Ok(vfs::Backend::MemFs));
}
