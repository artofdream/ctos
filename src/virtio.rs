//! virtio-mmio block on QEMU `virt` (Track A / A7 / ADR-028).
//!
//! Scan the virt transports, set up one split virtqueue, DMA at identity
//! PAs. Poll `used.idx`. Not virtio-pci. Not a virtio IRQ. Not FAT.

use core::fmt::Write;
use core::ptr::{addr_of, addr_of_mut};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use crate::paging;
use crate::timer;
use crate::uart;

const MMIO_BASE: usize = 0x0a00_0000;
const MMIO_STRIDE: usize = 0x200;
const MMIO_COUNT: usize = 32;
const MAGIC: u32 = 0x7472_6976; // "virt"
const VERSION_MODERN: u32 = 2;
const DEV_BLK: u32 = 2;

const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_DEV_FEAT: usize = 0x010;
const REG_DEV_FEAT_SEL: usize = 0x014;
const REG_DRV_FEAT: usize = 0x020;
const REG_DRV_FEAT_SEL: usize = 0x024;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_READY: usize = 0x03c;
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_STATUS: usize = 0x070;
const REG_DESC_LO: usize = 0x080;
const REG_DESC_HI: usize = 0x084;
const REG_AVAIL_LO: usize = 0x090;
const REG_AVAIL_HI: usize = 0x094;
const REG_USED_LO: usize = 0x0a0;
const REG_USED_HI: usize = 0x0a4;
const REG_CONFIG: usize = 0x100;

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

#[repr(C, align(16))]
struct Dma {
    desc: [Desc; QSZ],
    avail: Avail,
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

fn find_blk() -> Option<usize> {
    for i in 0..MMIO_COUNT {
        let base = MMIO_BASE + i * MMIO_STRIDE;
        unsafe {
            if mmio_read(base, REG_MAGIC) != MAGIC {
                continue;
            }
            if mmio_read(base, REG_VERSION) != VERSION_MODERN {
                continue;
            }
            if mmio_read(base, REG_DEVICE_ID) == DEV_BLK {
                return Some(base);
            }
        }
    }
    None
}

fn setup(base: usize) -> Option<u64> {
    unsafe {
        mmio_write(base, REG_STATUS, 0);
        dsb();
        mmio_write(base, REG_STATUS, STATUS_ACK | STATUS_DRIVER);

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
        mmio_write(base, REG_STATUS, STATUS_ACK | STATUS_DRIVER | STATUS_FEATURES_OK);
        if mmio_read(base, REG_STATUS) & STATUS_FEATURES_OK == 0 {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }

        mmio_write(base, REG_QUEUE_SEL, 0);
        let max = mmio_read(base, REG_QUEUE_NUM_MAX);
        if max < QSZ as u32 {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }
        mmio_write(base, REG_QUEUE_NUM, QSZ as u32);

        let dma = addr_of_mut!(DMA);
        (*dma) = Dma::ZERO;
        let desc_pa = dma_pa(addr_of!((*dma).desc) as *const u8);
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
        if mmio_read(base, REG_QUEUE_READY) != 1 {
            mmio_write(base, REG_STATUS, STATUS_FAILED);
            return None;
        }

        mmio_write(
            base,
            REG_STATUS,
            STATUS_ACK | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_OK,
        );

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
        cache_sync(addr_of!((*dma).desc) as u64, 4096);
        mmio_write(dev.base, REG_QUEUE_NOTIFY, 0);

        let timeout = timer::cntpct().saturating_add(timer_ticks(2));
        loop {
            cache_sync(addr_of!((*dma).used) as u64, 256);
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
    let Some(base) = find_blk() else {
        return;
    };
    let Some(cap) = setup(base) else {
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
