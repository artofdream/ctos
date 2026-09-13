//! OS image vs app payload slot (Track A / A9 / ADR-030).
//!
//! First cut plus leftover mile: the guest reads a **separate** app ELF
//! from FAT16 `/hello` (same VFS `open` as A7) and maps it with the A3
//! loader. A2–A4 now use this same FAT file (no `include_bytes!`).
//! This path has **no** embed fallback. Cross-update (same published
//! ELF on this OS and documented prior OS `ba6541c`) is a host smoke
//! probe, not a guest serial line. Not OTA. Not A-B flash. Not OCI.
//! Not app hosting.
//!
//! ADR-046: on the same boot, a **probe-only** CNTPCT trip maps the
//! same `hello-libctos.elf` bytes linked via `include_bytes!` (OUT_DIR
//! build artifact) so serial can print a compared pair. That probe is
//! **not** the A9 slot path and must never print `slot: embed`.

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

include!(concat!(env!("OUT_DIR"), "/hello_libctos_meta.rs"));

const _: () = assert!(HELLO_ELF_LEN > 64);
const _: () = assert!(HELLO_LEN > 0);
const _: () = assert!(HELLO_LOAD_VA == 0x8000_2000);

const SLOT_PATH: &str = "/hello";
const SLOT_MAX: usize = 64 * 1024;

/// Same bytes `build.rs` published for FAT `/hello`. Probe-only for
/// ADR-046 CNTPCT compare — not production A2–A4 / A9 load.
static EMBED_PROBE_ELF: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/hello-libctos.elf"));

static FAT_OK: AtomicBool = AtomicBool::new(false);
static LOAD_OK: AtomicBool = AtomicBool::new(false);
static APP_LOAD: AtomicU64 = AtomicU64::new(0);
static EMBED_LOAD: AtomicU64 = AtomicU64::new(0);

fn read_slot() -> Option<Vec<u8>> {
    let out = fat::read_file(SLOT_PATH, SLOT_MAX)?;
    if out.len() < 64 || out[0] != 0x7f || &out[1..4] != b"ELF" {
        return None;
    }
    Some(out)
}

/// Probe-only: map + `ERET` the linked-in hello ELF (no FAT read).
/// Prints `perf: embed-load`, never `slot: embed`.
fn measure_embed_load() -> Option<u64> {
    if EMBED_PROBE_ELF.len() != HELLO_ELF_LEN {
        return None;
    }
    if EMBED_PROBE_ELF[0] != 0x7f || &EMBED_PROBE_ELF[1..4] != b"ELF" {
        return None;
    }
    let t0 = timer::cntpct();
    if !loader::run_image(EMBED_PROBE_ELF) {
        return None;
    }
    let ticks = timer::cntpct().wrapping_sub(t0);
    if ticks == 0 {
        return None;
    }
    Some(ticks)
}

/// Serial proof: FAT `/hello` parsed and `ERET`ed. Not the A3 embed.
/// After the FAT trip, ADR-046 also times the linked-in bytes and
/// prints an honest pair (QEMU TCG lab — not a published bench).
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    FAT_OK.store(false, Ordering::SeqCst);
    LOAD_OK.store(false, Ordering::SeqCst);
    APP_LOAD.store(0, Ordering::SeqCst);
    EMBED_LOAD.store(0, Ordering::SeqCst);
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

    // ADR-046: same guest, probe-only linked-in trip for a compared pair.
    // Must not print `slot: embed` (production path stays FAT-only).
    if bytes.as_slice() != EMBED_PROBE_ELF {
        let _ = writeln!(w, "perf: embed-load missed");
        return false;
    }
    let Some(embed_ticks) = measure_embed_load() else {
        let _ = writeln!(w, "perf: embed-load missed");
        return false;
    };
    EMBED_LOAD.store(embed_ticks, Ordering::SeqCst);
    let _ = writeln!(w, "perf: embed-load ticks={embed_ticks}");
    let _ = writeln!(w, "perf: slot-delta app={ticks} embed={embed_ticks}");
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
    assert!(
        EMBED_LOAD.load(Ordering::SeqCst) > 0,
        "ADR-046 embed probe must advance CNTPCT"
    );
}

#[cfg(test)]
#[test_case]
fn slot_path_is_not_probe() {
    assert_eq!(SLOT_PATH, "/hello");
    assert!(vfs::valid_path(SLOT_PATH));
    assert!(!fat::has_name("/missing-slot"));
}

#[cfg(test)]
#[test_case]
fn slot_embed_probe_bytes_match_fat() {
    assert_eq!(EMBED_PROBE_ELF.len(), HELLO_ELF_LEN);
    assert_eq!(&EMBED_PROBE_ELF[0..4], b"\x7fELF");
    let fat = read_slot().expect("FAT /hello");
    assert_eq!(fat.as_slice(), EMBED_PROBE_ELF);
}
