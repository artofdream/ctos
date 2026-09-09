//! Physical 4 KiB frame allocator for QEMU `virt` (FR-09 / M7).
//!
//! The pool is the conventional virt RAM window after `__kernel_end`
//! (linker) up to `RAM_BASE + RAM_SIZE`. QEMU `-kernel` typically places
//! the DTB at `0x4000_0000` (below the kernel at `0x4008_0000`); that
//! region is never handed out. This is not a DTB memory-node parser and
//! not `GlobalAlloc` (M8). See ADR-008.

use spin::Mutex;

/// 4 KiB frames. Matches the TCR granule in `paging`.
pub const FRAME_SIZE: u64 = 4096;

/// QEMU `virt` RAM base (`hw/arm/virt.c` `VIRT_MEM`).
pub const VIRT_RAM_BASE: u64 = 0x4000_0000;

/// Default `-machine virt` RAM when `-m` is unset. Smoke pins `-m 128M`.
pub const VIRT_RAM_SIZE: u64 = 128 * 1024 * 1024;

/// Recycle this many freed frames. Overflow leaks — honest for M7.
const FREE_STACK: usize = 16;

unsafe extern "C" {
    static __kernel_end: u8;
}

struct FrameAlloc {
    start: u64,
    next: u64,
    end: u64,
    free: [u64; FREE_STACK],
    free_len: usize,
}

static ALLOC: Mutex<Option<FrameAlloc>> = Mutex::new(None);

fn kernel_end() -> u64 {
    core::ptr::addr_of!(__kernel_end) as usize as u64
}

fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// Walk from the first 4 KiB after the image/stacks to the virt RAM end.
pub fn init() {
    let start = align_up(kernel_end(), FRAME_SIZE);
    let end = VIRT_RAM_BASE + VIRT_RAM_SIZE;
    let mut g = ALLOC.lock();
    *g = Some(FrameAlloc {
        start,
        next: start,
        end,
        free: [0; FREE_STACK],
        free_len: 0,
    });
}

#[allow(dead_code)]
pub fn pool_start() -> u64 {
    ALLOC.lock().as_ref().map(|a| a.start).unwrap_or(0)
}

#[allow(dead_code)]
pub fn pool_end() -> u64 {
    ALLOC.lock().as_ref().map(|a| a.end).unwrap_or(0)
}

/// Next unused frame address (after recycled frees). For tests.
#[allow(dead_code)]
pub fn bump_next() -> u64 {
    ALLOC.lock().as_ref().map(|a| a.next).unwrap_or(0)
}

/// Allocate one 4 KiB frame. Recycled frees first, then the bump cursor.
pub fn alloc() -> Option<u64> {
    let mut g = ALLOC.lock();
    let a = g.as_mut()?;
    if a.free_len > 0 {
        a.free_len -= 1;
        return Some(a.free[a.free_len]);
    }
    if a.next.saturating_add(FRAME_SIZE) > a.end {
        return None;
    }
    let frame = a.next;
    a.next += FRAME_SIZE;
    Some(frame)
}

/// Return a frame to the small free stack. Extra frees are leaked.
pub fn free(frame: u64) {
    if frame & (FRAME_SIZE - 1) != 0 {
        return;
    }
    let mut g = ALLOC.lock();
    if let Some(a) = g.as_mut() {
        if a.free_len < FREE_STACK {
            a.free[a.free_len] = frame;
            a.free_len += 1;
        }
    }
}

#[cfg(test)]
#[test_case]
fn frame_alloc_aligned_and_distinct() {
    let a = alloc().expect("first frame");
    let b = alloc().expect("second frame");
    assert_eq!(a & (FRAME_SIZE - 1), 0);
    assert_eq!(b & (FRAME_SIZE - 1), 0);
    assert_ne!(a, b);
    assert!(a >= kernel_end());
    assert!(b >= kernel_end());
    assert!(a < VIRT_RAM_BASE + VIRT_RAM_SIZE);
    assert!(b < VIRT_RAM_BASE + VIRT_RAM_SIZE);
    free(b);
    free(a);
    let recycled = alloc().expect("recycled frame");
    assert!(recycled == a || recycled == b);
    free(recycled);
}

#[cfg(test)]
#[test_case]
fn frame_pool_is_after_kernel() {
    let start = align_up(kernel_end(), FRAME_SIZE);
    assert!(start >= kernel_end());
    assert_eq!(pool_end(), VIRT_RAM_BASE + VIRT_RAM_SIZE);
    assert!(bump_next() >= start);
    assert!(bump_next() <= pool_end());
}
