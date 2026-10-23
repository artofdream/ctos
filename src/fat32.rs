//! FAT32 read path on a second virtio-blk disk (Track A / ADR-107).
//!
//! Beyond FAT16 (ADR-028): parse the FAT32 BPB (`BPB_FATSz32`,
//! `BPB_RootClus`, `BPB_FSInfo`), check the FSInfo sector signatures, follow
//! cluster chains through the 28-bit FAT32 table, and walk the root
//! directory as a cluster chain (not a fixed region). Read-only. 8.3 names
//! only (LFN entries are skipped, not decoded). Not a VFS mount, not write,
//! not exFAT. The FAT16 disk and its VFS path (`fat.rs`) are unchanged.
//!
//! Fail closed: a volume whose cluster count is below 65525 is not FAT32
//! (it stays on the FAT16 path). A chain link that is free, reserved, bad,
//! out of range, cyclic, or whose length disagrees with the file size is an
//! error, never a short read. Host image: `mkfs.fat -F 32` via
//! `scripts/mkfat32.sh`.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::uart;
use crate::virtio;

const SECTOR: usize = 512;
/// Microsoft fatgen103: 65525 clusters or more is FAT32.
const FAT32_MIN_CLUSTERS: u32 = 65525;
const FAT32_MASK: u32 = 0x0FFF_FFFF;
const FAT32_BAD: u32 = 0x0FFF_FFF7;
const FAT32_EOC_MIN: u32 = 0x0FFF_FFF8;
const FSI_LEAD: u32 = 0x4161_5252;
const FSI_STRUC: u32 = 0x6141_7272;
const FSI_TRAIL: u32 = 0xAA55_0000;
const ATTR_LFN: u8 = 0x0F;
const ATTR_DIR: u8 = 0x10;
const ATTR_VOLUME: u8 = 0x08;

/// Probe files written by `scripts/mkfat32.sh` (8.3, no LFN).
const PROBE_NAME: &[u8; 11] = b"PROBE32 TXT";
const PROBE_BYTES: &[u8] = b"fat32-hi\n";
const BIG_NAME: &[u8; 11] = b"BIG     BIN";
const BIG_LEN: usize = 3000;
const LAST_NAME: &[u8; 11] = b"LAST    TXT";
const LAST_BYTES: &[u8] = b"fat32-root-chain\n";

pub type ReadFn = fn(u64, &mut [u8; SECTOR]) -> bool;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fat32Err {
    Io,
    NoSignature,
    BadGeometry,
    /// Cluster count < 65525: FAT12/16, not FAT32 (stays on `fat.rs`).
    NotFat32,
    BadFsInfo,
    /// Link is free (0), reserved (1), bad, or past the last cluster.
    BadLink,
    Cycle,
    SizeMismatch,
    NotFound,
    TooBig,
}

impl Fat32Err {
    pub fn reason(self) -> &'static str {
        match self {
            Fat32Err::Io => "io",
            Fat32Err::NoSignature => "no-signature",
            Fat32Err::BadGeometry => "bad-geometry",
            Fat32Err::NotFat32 => "not-fat32",
            Fat32Err::BadFsInfo => "bad-fsinfo",
            Fat32Err::BadLink => "bad-chain",
            Fat32Err::Cycle => "chain-cycle",
            Fat32Err::SizeMismatch => "size-mismatch",
            Fat32Err::NotFound => "not-found",
            Fat32Err::TooBig => "too-big",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Fat32 {
    read: ReadFn,
    spc: u32,
    reserved: u32,
    fat_sz: u32,
    first_data: u32,
    clusters: u32,
    root_cluster: u32,
    free_count: u32,
}

fn le16(b: &[u8], o: usize) -> u32 {
    u16::from_le_bytes([b[o], b[o + 1]]) as u32
}

fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

#[derive(Clone, Copy)]
pub struct Dirent {
    pub attr: u8,
    pub cluster: u32,
    pub size: u32,
}

impl Fat32 {
    pub fn mount(read: ReadFn) -> Result<Fat32, Fat32Err> {
        let mut b = [0u8; SECTOR];
        if !read(0, &mut b) {
            return Err(Fat32Err::Io);
        }
        if b[510] != 0x55 || b[511] != 0xAA {
            return Err(Fat32Err::NoSignature);
        }
        let bps = le16(&b, 11);
        let spc = b[13] as u32;
        let reserved = le16(&b, 14);
        let fats = b[16] as u32;
        let root_ents = le16(&b, 17);
        let tot16 = le16(&b, 19);
        let fat_sz16 = le16(&b, 22);
        let tot32 = le32(&b, 32);
        if bps != 512 || spc == 0 || !spc.is_power_of_two() || reserved == 0 || fats == 0 {
            return Err(Fat32Err::BadGeometry);
        }
        // FAT32 BPB: fixed root region and 16-bit FAT size must be zero.
        if root_ents != 0 || fat_sz16 != 0 {
            return Err(Fat32Err::NotFat32);
        }
        let total = if tot16 != 0 { tot16 } else { tot32 };
        let fat_sz = le32(&b, 36);
        let root_cluster = le32(&b, 44);
        let fsinfo = le16(&b, 48);
        if total == 0 || fat_sz == 0 {
            return Err(Fat32Err::BadGeometry);
        }
        let first_data = reserved
            .checked_add(fats.checked_mul(fat_sz).ok_or(Fat32Err::BadGeometry)?)
            .ok_or(Fat32Err::BadGeometry)?;
        if first_data >= total {
            return Err(Fat32Err::BadGeometry);
        }
        let clusters = (total - first_data) / spc;
        if clusters < FAT32_MIN_CLUSTERS {
            return Err(Fat32Err::NotFat32);
        }
        // Every cluster index must have a FAT32 slot in one FAT copy.
        if (clusters as u64 + 2) * 4 > fat_sz as u64 * SECTOR as u64 {
            return Err(Fat32Err::BadGeometry);
        }
        if root_cluster < 2 || root_cluster >= clusters + 2 {
            return Err(Fat32Err::BadGeometry);
        }
        if fsinfo == 0 || fsinfo >= reserved {
            return Err(Fat32Err::BadFsInfo);
        }
        let mut fi = [0u8; SECTOR];
        if !read(fsinfo as u64, &mut fi) {
            return Err(Fat32Err::Io);
        }
        if le32(&fi, 0) != FSI_LEAD || le32(&fi, 484) != FSI_STRUC || le32(&fi, 508) != FSI_TRAIL {
            return Err(Fat32Err::BadFsInfo);
        }
        let free_count = le32(&fi, 488);
        if free_count != 0xFFFF_FFFF && free_count > clusters {
            return Err(Fat32Err::BadFsInfo);
        }
        Ok(Fat32 {
            read,
            spc,
            reserved,
            fat_sz,
            first_data,
            clusters,
            root_cluster,
            free_count,
        })
    }

    pub fn clusters(&self) -> u32 {
        self.clusters
    }

    pub fn root_cluster(&self) -> u32 {
        self.root_cluster
    }

    pub fn free_count(&self) -> u32 {
        self.free_count
    }

    fn cluster_bytes(&self) -> u32 {
        self.spc * SECTOR as u32
    }

    fn valid_cluster(&self, c: u32) -> bool {
        c >= 2 && c < self.clusters + 2
    }

    /// Raw 28-bit FAT32 entry for `c` (first FAT copy).
    pub fn fat_entry(&self, c: u32) -> Result<u32, Fat32Err> {
        if !self.valid_cluster(c) {
            return Err(Fat32Err::BadLink);
        }
        let off = c as u64 * 4;
        let sec = self.reserved as u64 + off / SECTOR as u64;
        if sec >= self.reserved as u64 + self.fat_sz as u64 {
            return Err(Fat32Err::BadLink);
        }
        let mut b = [0u8; SECTOR];
        if !(self.read)(sec, &mut b) {
            return Err(Fat32Err::Io);
        }
        Ok(le32(&b, (off % SECTOR as u64) as usize) & FAT32_MASK)
    }

    /// Next cluster, `None` at end-of-chain; any other non-link is an error.
    fn next(&self, c: u32) -> Result<Option<u32>, Fat32Err> {
        let v = self.fat_entry(c)?;
        if v >= FAT32_EOC_MIN {
            return Ok(None);
        }
        if v == FAT32_BAD || !self.valid_cluster(v) {
            return Err(Fat32Err::BadLink);
        }
        Ok(Some(v))
    }

    /// Visit each cluster of the chain at `start`; `max` bounds cycles.
    fn walk(
        &self,
        start: u32,
        max: u32,
        mut f: impl FnMut(u32) -> Result<bool, Fat32Err>,
    ) -> Result<u32, Fat32Err> {
        if !self.valid_cluster(start) {
            return Err(Fat32Err::BadLink);
        }
        let mut c = start;
        let mut n = 0u32;
        loop {
            n += 1;
            if n > max {
                return Err(Fat32Err::Cycle);
            }
            if !f(c)? {
                return Ok(n);
            }
            match self.next(c)? {
                None => return Ok(n),
                Some(nx) => c = nx,
            }
        }
    }

    pub fn chain_len(&self, start: u32) -> Result<u32, Fat32Err> {
        self.walk(start, self.clusters, |_| Ok(true))
    }

    fn cluster_lba(&self, c: u32) -> u64 {
        self.first_data as u64 + (c as u64 - 2) * self.spc as u64
    }

    /// Scan the root directory chain. Returns (entry, root chain clusters).
    pub fn find_root(&self, name: &[u8; 11]) -> Result<(Dirent, u32), Fat32Err> {
        let mut found: Option<Dirent> = None;
        let mut ended = false;
        let n = self.walk(self.root_cluster, self.clusters, |c| {
            let base = self.cluster_lba(c);
            for s in 0..self.spc as u64 {
                let mut b = [0u8; SECTOR];
                if !(self.read)(base + s, &mut b) {
                    return Err(Fat32Err::Io);
                }
                for e in b.chunks_exact(32) {
                    if e[0] == 0 {
                        ended = true;
                        return Ok(false);
                    }
                    let attr = e[11];
                    if e[0] == 0xE5 || attr == ATTR_LFN || attr & ATTR_VOLUME != 0 {
                        continue;
                    }
                    if found.is_none() && &e[0..11] == name {
                        found = Some(Dirent {
                            attr,
                            cluster: (le16(e, 20) << 16) | le16(e, 26),
                            size: le32(e, 28),
                        });
                    }
                }
            }
            Ok(true)
        })?;
        let _ = ended;
        match found {
            Some(d) => Ok((d, n)),
            None => Err(Fat32Err::NotFound),
        }
    }

    /// Read a whole root file. Chain length must equal ceil(size/cluster).
    pub fn read_root_file(&self, name: &[u8; 11], max: usize) -> Result<Vec<u8>, Fat32Err> {
        let (d, _) = self.find_root(name)?;
        if d.attr & ATTR_DIR != 0 {
            return Err(Fat32Err::NotFound);
        }
        let size = d.size as usize;
        if size > max {
            return Err(Fat32Err::TooBig);
        }
        let cb = self.cluster_bytes() as usize;
        if size == 0 {
            return if d.cluster == 0 { Ok(Vec::new()) } else { Err(Fat32Err::SizeMismatch) };
        }
        let need = size.div_ceil(cb) as u32;
        let mut out = Vec::with_capacity(size);
        let got = self.walk(d.cluster, need, |c| {
            let base = self.cluster_lba(c);
            for s in 0..self.spc as u64 {
                if out.len() >= size {
                    break;
                }
                let mut b = [0u8; SECTOR];
                if !(self.read)(base + s, &mut b) {
                    return Err(Fat32Err::Io);
                }
                let take = (size - out.len()).min(SECTOR);
                out.extend_from_slice(&b[..take]);
            }
            Ok(true)
        });
        match got {
            // A chain longer than `need` trips the walk bound.
            Err(Fat32Err::Cycle) => return Err(Fat32Err::SizeMismatch),
            Err(e) => return Err(e),
            Ok(n) if n != need || out.len() != size => return Err(Fat32Err::SizeMismatch),
            Ok(_) => {}
        }
        Ok(out)
    }
}

/// Deterministic `BIG.BIN` byte `i` (matches `scripts/mkfat32.sh`).
pub fn big_byte(i: usize) -> u8 {
    ((i * 7 + 3) % 251) as u8
}

static OK: AtomicBool = AtomicBool::new(false);

fn disk_read(lba: u64, buf: &mut [u8; SECTOR]) -> bool {
    virtio::read_sector2(lba, buf)
}

fn run_probe() -> Result<(), Fat32Err> {
    let mut w = uart::raw();
    let v = Fat32::mount(disk_read)?;
    let _ = writeln!(
        w,
        "fat32: mount clusters={} root={} fsinfo=ok free={} disk-sectors={}",
        v.clusters(),
        v.root_cluster(),
        v.free_count(),
        virtio::capacity2()
    );
    let probe = v.read_root_file(PROBE_NAME, 64)?;
    if probe != PROBE_BYTES {
        return Err(Fat32Err::SizeMismatch);
    }
    let _ = writeln!(w, "fat32: read /probe32.txt bytes={}", probe.len());
    let (bd, _) = v.find_root(BIG_NAME)?;
    let big = v.read_root_file(BIG_NAME, 8192)?;
    if big.len() != BIG_LEN || big.iter().enumerate().any(|(i, b)| *b != big_byte(i)) {
        return Err(Fat32Err::SizeMismatch);
    }
    // Fragmented chain: count non-adjacent links (mkfat32 punches a hole).
    let mut prev = 0u32;
    let mut jumps = 0u32;
    v.walk(bd.cluster, v.clusters(), |c| {
        if prev != 0 && c != prev + 1 {
            jumps += 1;
        }
        prev = c;
        Ok(true)
    })?;
    let _ = writeln!(
        w,
        "fat32: chain /big.bin clusters={} jumps={} bytes={}",
        v.chain_len(bd.cluster)?,
        jumps,
        big.len()
    );
    if jumps == 0 {
        return Err(Fat32Err::BadLink);
    }
    let (_, root_n) = v.find_root(LAST_NAME)?;
    let last = v.read_root_file(LAST_NAME, 64)?;
    if last != LAST_BYTES || root_n < 2 {
        return Err(Fat32Err::NotFound);
    }
    let _ = writeln!(w, "fat32: root-chain clusters={} last=found", root_n);
    Ok(())
}

/// Serial proof (ADR-107): FAT32 BPB + FSInfo, FAT32 chains, root chain.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    OK.store(false, Ordering::SeqCst);
    if !virtio::ready2() {
        uart::write_str_raw("fat32: fail reason=no-disk\n");
        return false;
    }
    match run_probe() {
        Ok(()) => {
            uart::write_str_raw("fat32: ok\n");
            OK.store(true, Ordering::SeqCst);
            true
        }
        Err(e) => {
            let mut w = uart::raw();
            let _ = writeln!(w, "fat32: fail reason={}", e.reason());
            false
        }
    }
}

// ---------------------------------------------------------------------------
// In-memory negatives: a tiny synthetic FAT32 image (no disk), so corrupt
// chains are exercised without a second QEMU run.

#[cfg(test)]
mod mem {
    use super::*;
    use spin::Mutex;

    pub static IMG: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    // Compact backing: [boot, fsinfo, FAT sector 0, 16 data clusters].
    // Every other sector reads as zeros (sparse), so the 64 KiB heap is enough.
    pub fn read(lba: u64, buf: &mut [u8; SECTOR]) -> bool {
        let img = IMG.lock();
        let l = lba as u32;
        let slot = if l < 2 {
            Some(l as usize)
        } else if l == RES {
            Some(2)
        } else if l >= FIRST_DATA && l < FIRST_DATA + 16 {
            Some(3 + (l - FIRST_DATA) as usize)
        } else {
            None
        };
        match slot {
            Some(i) if (i + 1) * SECTOR <= img.len() => {
                buf.copy_from_slice(&img[i * SECTOR..(i + 1) * SECTOR]);
            }
            Some(_) => return false,
            None => buf.fill(0),
        }
        true
    }

    // Geometry: 1 sector/cluster, 32 reserved, 1 FAT of 520 sectors,
    // clusters = 66000 (>= 65525).
    pub const RES: u32 = 32;
    pub const FATSZ: u32 = 520;
    pub const CLUSTERS: u32 = 66000;
    pub const FIRST_DATA: u32 = RES + FATSZ;
    pub const BACKED: usize = 3 + 16;

    /// Only clusters < 128 (FAT sector 0) are backed.
    pub fn set_fat(img: &mut [u8], c: u32, v: u32) {
        let off = 2 * SECTOR + c as usize * 4;
        img[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn lba(c: u32) -> usize {
        (3 + c as usize - 2) * SECTOR
    }

    pub fn dirent(img: &mut [u8], cl: u32, idx: usize, name: &[u8; 11], start: u32, size: u32) {
        let o = lba(cl) + idx * 32;
        img[o..o + 11].copy_from_slice(name);
        img[o + 11] = 0x20;
        img[o + 20..o + 22].copy_from_slice(&((start >> 16) as u16).to_le_bytes());
        img[o + 26..o + 28].copy_from_slice(&(start as u16).to_le_bytes());
        img[o + 28..o + 32].copy_from_slice(&size.to_le_bytes());
    }

    /// Root = clusters 2 -> 3. File `F.BIN` (700 B) = clusters 5 -> 4.
    pub fn build() {
        let mut img = alloc::vec![0u8; BACKED * SECTOR];
        let b = &mut img[0..SECTOR];
        b[11..13].copy_from_slice(&512u16.to_le_bytes());
        b[13] = 1;
        b[14..16].copy_from_slice(&(RES as u16).to_le_bytes());
        b[16] = 1;
        let total = FIRST_DATA + CLUSTERS;
        b[32..36].copy_from_slice(&total.to_le_bytes());
        b[36..40].copy_from_slice(&FATSZ.to_le_bytes());
        b[44..48].copy_from_slice(&2u32.to_le_bytes());
        b[48..50].copy_from_slice(&1u16.to_le_bytes());
        b[510] = 0x55;
        b[511] = 0xAA;
        let f = &mut img[SECTOR..2 * SECTOR];
        f[0..4].copy_from_slice(&FSI_LEAD.to_le_bytes());
        f[484..488].copy_from_slice(&FSI_STRUC.to_le_bytes());
        f[488..492].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        f[508..512].copy_from_slice(&FSI_TRAIL.to_le_bytes());
        set_fat(&mut img, 0, 0x0FFF_FFF8);
        set_fat(&mut img, 1, 0x0FFF_FFFF);
        set_fat(&mut img, 2, 3);
        set_fat(&mut img, 3, 0x0FFF_FFFF);
        set_fat(&mut img, 5, 4);
        set_fat(&mut img, 4, 0x0FFF_FFFF);
        // Fill root cluster 2 with 16 used entries so `F.BIN` lands in cluster 3.
        for i in 0..16 {
            let mut n = *b"PAD00   BIN";
            n[3] = b'0' + (i / 10) as u8;
            n[4] = b'0' + (i % 10) as u8;
            dirent(&mut img, 2, i, &n, 0, 0);
        }
        dirent(&mut img, 3, 0, b"F       BIN", 5, 700);
        for i in 0..700 {
            let o = if i < 512 { lba(5) + i } else { lba(4) + i - 512 };
            img[o] = big_byte(i);
        }
        *IMG.lock() = img;
    }
}

#[cfg(test)]
#[test_case]
fn fat32_mem_reads_file_through_root_chain() {
    mem::build();
    let v = Fat32::mount(mem::read).expect("synthetic FAT32 must mount");
    assert_eq!(v.clusters(), mem::CLUSTERS);
    let (d, root_n) = v.find_root(b"F       BIN").expect("F.BIN is in root cluster 3");
    assert_eq!(root_n, 2);
    assert_eq!(v.chain_len(d.cluster), Ok(2));
    let data = v.read_root_file(b"F       BIN", 4096).expect("read F.BIN");
    assert_eq!(data.len(), 700);
    assert!(data.iter().enumerate().all(|(i, b)| *b == big_byte(i)));
}

#[cfg(test)]
#[test_case]
fn fat32_mem_free_link_fails_closed() {
    mem::build();
    mem::set_fat(&mut mem::IMG.lock(), 5, 0); // chain points at "free"
    let v = Fat32::mount(mem::read).unwrap();
    assert_eq!(v.read_root_file(b"F       BIN", 4096), Err(Fat32Err::BadLink));
}

#[cfg(test)]
#[test_case]
fn fat32_mem_cycle_fails_closed() {
    mem::build();
    mem::set_fat(&mut mem::IMG.lock(), 3, 2); // root chain 2 -> 3 -> 2
    let v = Fat32::mount(mem::read).unwrap();
    assert_eq!(v.chain_len(2), Err(Fat32Err::Cycle));
}

#[cfg(test)]
#[test_case]
fn fat32_mem_bad_and_out_of_range_links() {
    mem::build();
    mem::set_fat(&mut mem::IMG.lock(), 5, 0x0FFF_FFF7);
    let v = Fat32::mount(mem::read).unwrap();
    assert_eq!(v.read_root_file(b"F       BIN", 4096), Err(Fat32Err::BadLink));
    mem::set_fat(&mut mem::IMG.lock(), 5, mem::CLUSTERS + 2);
    let v = Fat32::mount(mem::read).unwrap();
    assert_eq!(v.read_root_file(b"F       BIN", 4096), Err(Fat32Err::BadLink));
}

#[cfg(test)]
#[test_case]
fn fat32_mem_size_mismatch_fails_closed() {
    mem::build();
    // Chain 5 -> 4 -> 6 -> EOC is longer than ceil(700/512) = 2.
    {
        let mut img = mem::IMG.lock();
        mem::set_fat(&mut img, 4, 6);
        mem::set_fat(&mut img, 6, 0x0FFF_FFFF);
    }
    let v = Fat32::mount(mem::read).unwrap();
    assert_eq!(v.read_root_file(b"F       BIN", 4096), Err(Fat32Err::SizeMismatch));
}

#[cfg(test)]
#[test_case]
fn fat32_mem_bad_fsinfo_and_fat16_geometry_rejected() {
    mem::build();
    mem::IMG.lock()[SECTOR + 484] ^= 0xFF;
    assert_eq!(Fat32::mount(mem::read).err(), Some(Fat32Err::BadFsInfo));
    mem::build();
    // Shrink to a FAT16-sized cluster count: not FAT32, fail closed.
    let total = mem::FIRST_DATA + 60000;
    mem::IMG.lock()[32..36].copy_from_slice(&total.to_le_bytes());
    assert_eq!(Fat32::mount(mem::read).err(), Some(Fat32Err::NotFat32));
    mem::build();
    mem::IMG.lock()[17] = 0; // root_ents low byte
    mem::IMG.lock()[18] = 2; // non-zero fixed root = FAT16 layout
    assert_eq!(Fat32::mount(mem::read).err(), Some(Fat32Err::NotFat32));
}

#[cfg(test)]
#[test_case]
fn fat32_real_disk_probe() {
    assert!(virtio::ready2(), "second virtio-blk (FAT32 image) must be attached");
    assert!(observe_probe(), "FAT32 disk probe must print fat32: ok");
    assert!(OK.load(Ordering::SeqCst));
    // FAT16 primary stays FAT16 and is not mounted by the FAT32 reader.
    assert_eq!(Fat32::mount(crate::virtio::read_sector).err(), Some(Fat32Err::NotFat32));
}
