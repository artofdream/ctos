//! OS image vs app payload slot (Track A / A9 / ADR-030).
//!
//! First cut plus leftover mile: the guest reads a **separate** app ELF
//! from FAT16 `/hello` (same VFS `open` as A7) and maps it with the A3
//! loader. A2–A4 now use this same FAT file (no `include_bytes!`).
//! This path has **no** embed fallback. Cross-update (same published
//! ELF on this OS and documented prior OS `ba6541c`) is a host smoke
//! probe, not a guest serial line. Not OTA. Not A-B flash. Not OCI.
//! Not app hosting.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::fat;
use crate::loader;
use crate::paging;
use crate::timer;
use crate::uart;
#[cfg(test)]
use crate::vfs;

const SLOT_PATH: &str = "/hello";
const SLOT_MAX: usize = 64 * 1024;

static FAT_OK: AtomicBool = AtomicBool::new(false);
static LOAD_OK: AtomicBool = AtomicBool::new(false);
static APP_LOAD: AtomicU64 = AtomicU64::new(0);

fn read_slot() -> Option<Vec<u8>> {
    let out = fat::read_file(SLOT_PATH, SLOT_MAX)?;
    if out.len() < 64 || out[0] != 0x7f || &out[1..4] != b"ELF" {
        return None;
    }
    Some(out)
}

/// Serial proof: FAT `/hello` parsed and `ERET`ed. Not the A3 embed.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    FAT_OK.store(false, Ordering::SeqCst);
    LOAD_OK.store(false, Ordering::SeqCst);
    APP_LOAD.store(0, Ordering::SeqCst);
    if !paging::mmu_enabled() || !paging::user_map_ready() || !fat::mounted() {
        return false;
    }
    if !fat::has_name(SLOT_PATH) {
        return false;
    }
    let t0 = timer::cntpct();
    let Some(bytes) = read_slot() else {
        return false;
    };
    let Ok(img) = loader::parse_elf64(&bytes) else {
        return false;
    };
    let mut w = uart::raw();
    let _ = writeln!(w, "slot: fat bytes={}", bytes.len());
    FAT_OK.store(true, Ordering::SeqCst);
    if !loader::run_image(&bytes) {
        return false;
    }
    let ticks = timer::cntpct().wrapping_sub(t0);
    if ticks == 0 {
        return false;
    }
    APP_LOAD.store(ticks, Ordering::SeqCst);
    let _ = writeln!(w, "slot: mapped segs={} entry={:#x}", img.nseg, img.entry);
    let _ = writeln!(w, "slot: ok");
    let _ = writeln!(w, "perf: app-load ticks={ticks}");
    LOAD_OK.store(true, Ordering::SeqCst);
    true
}

#[cfg(test)]
#[test_case]
fn slot_fat_hello_is_elf() {
    assert!(fat::mounted(), "FAT16 must mount so /hello can exist");
    assert!(fat::has_name(SLOT_PATH), "A9 app slot is FAT /hello");
    let bytes = read_slot().expect("VFS /hello must be an ELF");
    assert_eq!(&bytes[0..4], b"\x7fELF");
    let img = loader::parse_elf64(&bytes).expect("slot ELF must parse");
    assert_eq!(img.entry, paging::EL0_PAGE);
    assert!(img.nseg >= 1);
}

#[cfg(test)]
#[test_case]
fn slot_load_from_fat_erets() {
    assert!(paging::mmu_enabled());
    assert!(
        observe_probe(),
        "A9 slot must load FAT /hello, ERET, and print slot: ok"
    );
    assert!(FAT_OK.load(Ordering::SeqCst));
    assert!(LOAD_OK.load(Ordering::SeqCst));
    assert!(APP_LOAD.load(Ordering::SeqCst) > 0);
}

#[cfg(test)]
#[test_case]
fn slot_path_is_not_probe() {
    assert_eq!(SLOT_PATH, "/hello");
    assert!(vfs::valid_path(SLOT_PATH));
    assert!(!fat::has_name("/missing-slot"));
}
