//! Kernel heap: first-fit free list on identity-mapped frames (FR-10 / M8).
//!
//! After the EL1 identity map is on, `init` takes a contiguous run of
//! frames from the bump pool and treats that VA==PA window as the heap.
//! `Box` / `Vec` go through `GlobalAlloc`. Not a growing heap, not a
//! userspace allocator, not the scheduler. See ADR-009.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::fmt::Write;
use core::ptr::{self, NonNull};
use spin::Mutex;

use crate::frame;
use crate::uart;

/// 16 frames = 64 KiB. Enough for Box/Vec smoke; not a general size claim.
pub const HEAP_FRAMES: usize = 16;
pub const HEAP_SIZE: usize = HEAP_FRAMES * frame::FRAME_SIZE as usize;

const ALIGN: usize = 16;
const PROBE: u64 = 0x4354_4F53; // "CTOS"

#[repr(C, align(16))]
struct Header {
    size: usize,
    next: Option<NonNull<Header>>,
}

const HEADER_SIZE: usize = core::mem::size_of::<Header>();

struct Heap {
    base: usize,
    end: usize,
    free: Option<NonNull<Header>>,
}

// Single-core: `spin::Mutex` is the only accessor. `NonNull` is !Send.
unsafe impl Send for Heap {}

impl Heap {
    const fn empty() -> Self {
        Self {
            base: 0,
            end: 0,
            free: None,
        }
    }

    fn init(&mut self, base: usize, size: usize) {
        if base == 0 || size < HEADER_SIZE + ALIGN {
            return;
        }
        self.base = base;
        self.end = base + size;
        let h = base as *mut Header;
        unsafe {
            h.write(Header {
                size,
                next: None,
            });
            self.free = Some(NonNull::new_unchecked(h));
        }
    }

    fn ready(&self) -> bool {
        self.base != 0
    }

    fn header_pad(align: usize) -> usize {
        align_up(HEADER_SIZE, align.max(ALIGN))
    }

    fn need(layout: Layout) -> (usize, usize) {
        let align = layout.align().max(ALIGN);
        let payload = align_up(layout.size().max(1), align);
        let pad = Self::header_pad(align);
        (pad + payload, pad)
    }

    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        if !self.ready() {
            return ptr::null_mut();
        }
        let (need, pad) = Self::need(layout);
        let mut prev: Option<NonNull<Header>> = None;
        let mut cur = self.free;
        while let Some(node) = cur {
            let h = node.as_ptr();
            let size = unsafe { (*h).size };
            let next = unsafe { (*h).next };
            if size >= need {
                let leftover = size - need;
                if leftover >= HEADER_SIZE + ALIGN {
                    let split = (h as usize + need) as *mut Header;
                    unsafe {
                        split.write(Header {
                            size: leftover,
                            next,
                        });
                        (*h).size = need;
                        (*h).next = None;
                    }
                    self.unlink(prev, Some(unsafe { NonNull::new_unchecked(split) }));
                } else {
                    unsafe {
                        (*h).next = None;
                    }
                    self.unlink(prev, next);
                }
                return (h as usize + pad) as *mut u8;
            }
            prev = Some(node);
            cur = next;
        }
        ptr::null_mut()
    }

    fn unlink(&mut self, prev: Option<NonNull<Header>>, new_next: Option<NonNull<Header>>) {
        match prev {
            Some(p) => unsafe { (*p.as_ptr()).next = new_next },
            None => self.free = new_next,
        }
    }

    fn dealloc(&mut self, payload: *mut u8, layout: Layout) {
        if payload.is_null() || !self.ready() {
            return;
        }
        let (_, pad) = Self::need(layout);
        let h = (payload as usize).wrapping_sub(pad) as *mut Header;
        let addr = h as usize;
        if addr < self.base || addr >= self.end {
            return;
        }
        self.insert_coalesce(h);
    }

    /// Address-sorted insert, then merge with predecessor and successor.
    fn insert_coalesce(&mut self, new: *mut Header) {
        let new_addr = new as usize;
        let mut prev: Option<NonNull<Header>> = None;
        let mut succ = self.free;
        while let Some(n) = succ {
            if n.as_ptr() as usize >= new_addr {
                break;
            }
            prev = Some(n);
            succ = unsafe { (*n.as_ptr()).next };
        }

        unsafe {
            (*new).next = succ;
        }
        match prev {
            Some(p) => unsafe { (*p.as_ptr()).next = Some(NonNull::new_unchecked(new)) },
            None => self.free = Some(unsafe { NonNull::new_unchecked(new) }),
        }

        if let Some(s) = succ {
            if new_addr + unsafe { (*new).size } == s.as_ptr() as usize {
                unsafe {
                    (*new).size += (*s.as_ptr()).size;
                    (*new).next = (*s.as_ptr()).next;
                }
            }
        }

        if let Some(p) = prev {
            let p_ptr = p.as_ptr();
            if p_ptr as usize + unsafe { (*p_ptr).size } == new_addr {
                unsafe {
                    (*p_ptr).size += (*new).size;
                    (*p_ptr).next = (*new).next;
                }
            }
        }
    }
}

fn align_up(n: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    n.saturating_add(align - 1) & !(align - 1)
}

struct LockedHeap(Mutex<Heap>);

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap(Mutex::new(Heap::empty()));

/// Take a contiguous frame run and install one free block. MMU must be on.
pub fn init() {
    let Some(base) = frame::alloc_contiguous(HEAP_FRAMES) else {
        return;
    };
    ALLOCATOR.0.lock().init(base as usize, HEAP_SIZE);
}

#[allow(dead_code)] // `#[test_case]` + hello probe.
pub fn is_ready() -> bool {
    ALLOCATOR.0.lock().ready()
}

#[allow(dead_code)]
pub fn heap_base() -> u64 {
    ALLOCATOR.0.lock().base as u64
}

#[allow(dead_code)]
pub fn heap_end() -> u64 {
    ALLOCATOR.0.lock().end as u64
}

/// Serial proof: `Box` write/read/drop/reuse + `Vec` grow.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !is_ready() {
        return false;
    }
    let boxed = Box::new(PROBE);
    if *boxed != PROBE {
        return false;
    }
    let p1 = Box::into_raw(boxed);
    unsafe {
        drop(Box::from_raw(p1));
    }
    let boxed2 = Box::new(PROBE.wrapping_add(1));
    if *boxed2 != PROBE.wrapping_add(1) {
        return false;
    }
    let p2 = Box::into_raw(boxed2);
    if p1 != p2 {
        unsafe {
            drop(Box::from_raw(p2));
        }
        return false;
    }
    unsafe {
        drop(Box::from_raw(p2));
    }

    let mut v: Vec<u64> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    if v.len() != 3 || v[0] != 1 || v[2] != 3 {
        return false;
    }
    let addr = v.as_ptr() as u64;
    if addr < heap_base() || addr >= heap_end() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "heap: ok");
    true
}

#[cfg(test)]
#[test_case]
fn heap_lives_in_frame_pool() {
    assert!(is_ready());
    let base = heap_base();
    assert_eq!(base & (frame::FRAME_SIZE - 1), 0);
    assert!(base >= frame::pool_start());
    assert_eq!(heap_end(), base + HEAP_SIZE as u64);
    assert!(heap_end() <= frame::pool_end());
}

#[cfg(test)]
#[test_case]
fn box_alloc_roundtrip() {
    let b = Box::new(PROBE);
    assert_eq!(*b, PROBE);
    let p = Box::into_raw(b);
    unsafe {
        drop(Box::from_raw(p));
    }
    let b2 = Box::new(PROBE.wrapping_add(1));
    assert_eq!(*b2, PROBE.wrapping_add(1));
    let p2 = Box::into_raw(b2);
    assert_eq!(p, p2, "first-fit should reuse the freed Box");
    unsafe {
        drop(Box::from_raw(p2));
    }
}

#[cfg(test)]
#[test_case]
fn vec_grows() {
    let mut v = Vec::new();
    for i in 0..32u64 {
        v.push(i);
    }
    assert_eq!(v.len(), 32);
    assert_eq!(v[0], 0);
    assert_eq!(v[31], 31);
    let addr = v.as_ptr() as u64;
    assert!(addr >= heap_base());
    assert!(addr < heap_end());
}
