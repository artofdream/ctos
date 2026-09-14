//! Second freestanding sample slot: FAT `/fsdemo` → `fs-libctos` (ADR-059).
//!
//! Loads the published ELF beside `/hello`, maps with the A3 loader, and
//! `ERET`s. The payload exercises libctos VFS wrappers (`/memdemo` → memfs
//! via ADR-058 prefix mounts). Not POSIX. Not getdents. Not app-hosting reopen.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fat;
use crate::loader;
use crate::paging;
use crate::uart;
use crate::vfs;

include!(concat!(env!("OUT_DIR"), "/fs_libctos_meta.rs"));

const _: () = assert!(FSDEMO_ELF_LEN > 64);
const _: () = assert!(FSDEMO_LOAD_VA == 0x8000_2000);

const SLOT_PATH: &str = "/fsdemo";
const SLOT_MAX: usize = 64 * 1024;
/// Last `uart_write` from `user/fs-libctos` is `libctos: fs-ok\n` (15 bytes).
const FSDEMO_UART_LEN: u64 = 15;

static FAT_OK: AtomicBool = AtomicBool::new(false);
static LOAD_OK: AtomicBool = AtomicBool::new(false);

fn read_slot() -> Option<Vec<u8>> {
    let out = fat::read_file(SLOT_PATH, SLOT_MAX)?;
    if out.len() != FSDEMO_ELF_LEN || out[0] != 0x7f || &out[1..4] != b"ELF" {
        return None;
    }
    Some(out)
}

/// Serial proof: FAT `/fsdemo` parsed and `ERET`ed. Keep `/hello` intact.
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
    // Idempotent for #[test_case] re-entry: drop prior memfs /memdemo if any.
    let _ = vfs::unlink("/memdemo");
    let mut w = uart::raw();
    let _ = writeln!(w, "fsdemo: fat bytes={}", bytes.len());
    FAT_OK.store(true, Ordering::SeqCst);
    if !loader::run_image_expecting(&bytes, FSDEMO_UART_LEN) {
        return false;
    }
    let _ = writeln!(w, "fsdemo: mapped segs={} entry={:#x}", img.nseg, img.entry);
    let _ = writeln!(w, "fsdemo: ok");
    LOAD_OK.store(true, Ordering::SeqCst);
    true
}

#[cfg(test)]
#[test_case]
fn fsdemo_fat_is_elf() {
    assert!(fat::mounted(), "FAT16 must mount so /fsdemo can exist");
    assert!(fat::has_name(SLOT_PATH), "ADR-059 sample slot is FAT /fsdemo");
    let bytes = read_slot().expect("VFS /fsdemo must be an ELF");
    assert_eq!(&bytes[0..4], b"\x7fELF");
    let img = loader::parse_elf64(&bytes).expect("fsdemo ELF must parse");
    assert_eq!(img.entry, paging::EL0_PAGE);
    assert!(img.nseg >= 1);
}

#[cfg(test)]
#[test_case]
fn fsdemo_load_from_fat_erets() {
    assert!(paging::mmu_enabled());
    assert!(
        observe_probe(),
        "fsdemo slot must load FAT /fsdemo, ERET, and print fsdemo: ok"
    );
    assert!(FAT_OK.load(Ordering::SeqCst));
    assert!(LOAD_OK.load(Ordering::SeqCst));
}

#[cfg(test)]
#[test_case]
fn fsdemo_path_is_memdemo_mount() {
    assert_eq!(SLOT_PATH, "/fsdemo");
    assert!(vfs::valid_path(SLOT_PATH));
    assert!(vfs::valid_path("/memdemo"));
    assert_eq!(vfs::resolve("/fsdemo"), Ok(vfs::Backend::Fat16));
    assert_eq!(vfs::resolve("/memdemo"), Ok(vfs::Backend::MemFs));
}
