//! FAT16 on virtio-blk, behind the thin VFS (Track A / A7 / ADR-028 + ADR-050
//! + ADR-056 + ADR-057 + ADR-064).
//!
//! Read + write of root files. Known `/probe` (8.3 `PROBE`) and A9 `/hello`
//! (8.3 `HELLO`, app ELF). Guest may create a root file and grow it across
//! multiple clusters (ADR-064). Root listing (`readdir`) returns thin-VFS
//! paths. Guest may delete a root file (`unlink`) by marking the dirent
//! deleted and freeing its cluster chain. Not FAT32. Not xv6. Not POSIX
//! `unlink` / `getdents` / `opendir` / full write API. Host image is
//! `scripts/mkfat16.py`.
//!
//! `vfs::create` stays memfs-first for A6 probe names; FAT paths under `/`
//! route to the FAT backend ([ADR-058](../docs/03-adr/ADR-058-vfs-prefix-mounts.md)).
//! Write on an open FAT handle is the same `vfs::write`; delete is
//! `fat::unlink` / `vfs::unlink`.
//!
//! ADR-051: CNTPCT around the FAT VFS write of the small `/probe` payload,
//! compared on the same boot to a memfs write of the same bytes (raw ticks;
//! not a bench / percent).
//!
//! ADR-065: same-boot CNTPCT pair around FAT vs memfs **read** of the small
//! `/probe` payload (`fat-hi`) — raw ticks; not a bench / percent.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use crate::timer;
use crate::uart;
use crate::vfs::{self, FsError, VfsOps, FILE_MAX, PATH_MAX};
use crate::virtio;

const SECTOR: usize = 512;
const MAX_HANDLES: usize = 4;
const PROBE_PATH: &str = "/probe";
const PROBE_BYTES: &[u8] = b"fat-hi";
const WRITE_BYTES: &[u8] = b"fat-wr";
const CREATE_PATH: &str = "/fwr";
const CREATE_BYTES: &[u8] = b"fat-nw";
const DELETE_PATH: &str = "/fdel";
const DELETE_BYTES: &[u8] = b"fat-dl";
/// Multi-cluster grow probe (ADR-064). Length must exceed one cluster (512).
const GROW_PATH: &str = "/fgrow";
const GROW_LEN: usize = 600;
/// Cap on names returned by one root `readdir` (ADR-056).
pub const READDIR_MAX: usize = 16;
/// Memfs path for the ADR-051 compared write (same payload as WRITE_BYTES).
const MEMFS_CMP_PATH: &str = "/mwprobe";
/// Memfs path for the ADR-065 compared read (same payload as PROBE_BYTES).
const MEMFS_READ_CMP_PATH: &str = "/mrprobe";
const EOC: u16 = 0xFFFF;
const ATTR_ARCHIVE: u8 = 0x20;

#[derive(Clone, Copy)]
struct FatHandle {
    used: bool,
    name: [u8; 11],
    cluster: u16,
    size: u32,
    off: u32,
}

impl FatHandle {
    const fn empty() -> Self {
        Self {
            used: false,
            name: [0; 11],
            cluster: 0,
            size: 0,
            off: 0,
        }
    }
}

struct Fat16 {
    mounted: bool,
    bps: u16,
    spc: u8,
    reserved: u16,
    fats: u8,
    fat_sz: u16,
    root_ents: u16,
    first_data: u32,
    root_sec: u32,
    total_sec: u32,
    handles: [FatHandle; MAX_HANDLES],
}

impl Fat16 {
    const fn empty() -> Self {
        Self {
            mounted: false,
            bps: 0,
            spc: 0,
            reserved: 0,
            fats: 0,
            fat_sz: 0,
            root_ents: 0,
            first_data: 0,
            root_sec: 0,
            total_sec: 0,
            handles: [
                FatHandle::empty(),
                FatHandle::empty(),
                FatHandle::empty(),
                FatHandle::empty(),
            ],
        }
    }

    fn read_sector(&self, lba: u32, buf: &mut [u8; SECTOR]) -> bool {
        virtio::read_sector(lba as u64, buf)
    }

    fn write_sector(&self, lba: u32, buf: &[u8; SECTOR]) -> bool {
        virtio::write_sector(lba as u64, buf)
    }

    fn cluster_bytes(&self) -> u32 {
        self.bps as u32 * self.spc as u32
    }

    fn clusters(&self) -> u32 {
        if self.first_data >= self.total_sec || self.spc == 0 {
            return 0;
        }
        (self.total_sec - self.first_data) / self.spc as u32
    }

    fn fat_entry(&self, cluster: u16) -> Option<u16> {
        if cluster < 2 {
            return None;
        }
        let off = (cluster as u32) * 2;
        let sec = self.reserved as u32 + off / self.bps as u32;
        let ent = (off % self.bps as u32) as usize;
        let mut buf = [0u8; SECTOR];
        if !self.read_sector(sec, &mut buf) {
            return None;
        }
        Some(u16::from_le_bytes([buf[ent], buf[ent + 1]]))
    }

    fn set_fat_entry(&self, cluster: u16, value: u16) -> bool {
        if cluster < 2 || self.fats == 0 || self.fat_sz == 0 {
            return false;
        }
        let off = (cluster as u32) * 2;
        let within = off / self.bps as u32;
        let ent = (off % self.bps as u32) as usize;
        let bytes = value.to_le_bytes();
        for f in 0..self.fats as u32 {
            let sec = self.reserved as u32 + f * self.fat_sz as u32 + within;
            let mut buf = [0u8; SECTOR];
            if !self.read_sector(sec, &mut buf) {
                return false;
            }
            buf[ent] = bytes[0];
            buf[ent + 1] = bytes[1];
            if !self.write_sector(sec, &buf) {
                return false;
            }
        }
        true
    }

    fn cluster_lba(&self, cluster: u16) -> u32 {
        self.first_data + (cluster as u32 - 2) * self.spc as u32
    }

    fn lookup(&self, name83: &[u8; 11]) -> Option<(u16, u32)> {
        if !self.mounted {
            return None;
        }
        let root_secs = (self.root_ents as u32 * 32 + self.bps as u32 - 1) / self.bps as u32;
        let mut buf = [0u8; SECTOR];
        for s in 0..root_secs {
            if !self.read_sector(self.root_sec + s, &mut buf) {
                return None;
            }
            for i in 0..(self.bps as usize / 32) {
                let e = &buf[i * 32..i * 32 + 32];
                if e[0] == 0 {
                    return None;
                }
                if e[0] == 0xE5 {
                    continue;
                }
                let attr = e[11];
                if attr & 0x08 != 0 || attr & 0x0F == 0x0F || attr & 0x10 != 0 {
                    continue;
                }
                if &e[0..11] == name83 {
                    let cluster = u16::from_le_bytes([e[26], e[27]]);
                    let size = u32::from_le_bytes([e[28], e[29], e[30], e[31]]);
                    return Some((cluster, size));
                }
            }
        }
        None
    }

    fn update_dirent_size(&self, name83: &[u8; 11], size: u32) -> bool {
        let root_secs = (self.root_ents as u32 * 32 + self.bps as u32 - 1) / self.bps as u32;
        let mut buf = [0u8; SECTOR];
        for s in 0..root_secs {
            if !self.read_sector(self.root_sec + s, &mut buf) {
                return false;
            }
            for i in 0..(self.bps as usize / 32) {
                let off = i * 32;
                let e = &buf[off..off + 32];
                if e[0] == 0 {
                    return false;
                }
                if e[0] == 0xE5 {
                    continue;
                }
                let attr = e[11];
                if attr & 0x08 != 0 || attr & 0x0F == 0x0F || attr & 0x10 != 0 {
                    continue;
                }
                if &e[0..11] == name83 {
                    let sz = size.to_le_bytes();
                    buf[off + 28..off + 32].copy_from_slice(&sz);
                    return self.write_sector(self.root_sec + s, &buf);
                }
            }
        }
        false
    }

    /// Mark a root dirent deleted (first byte `0xE5`). Returns (cluster, size).
    fn mark_dirent_deleted(&self, name83: &[u8; 11]) -> Option<(u16, u32)> {
        let root_secs = (self.root_ents as u32 * 32 + self.bps as u32 - 1) / self.bps as u32;
        let mut buf = [0u8; SECTOR];
        for s in 0..root_secs {
            if !self.read_sector(self.root_sec + s, &mut buf) {
                return None;
            }
            for i in 0..(self.bps as usize / 32) {
                let off = i * 32;
                let e = &buf[off..off + 32];
                if e[0] == 0 {
                    return None;
                }
                if e[0] == 0xE5 {
                    continue;
                }
                let attr = e[11];
                if attr & 0x08 != 0 || attr & 0x0F == 0x0F || attr & 0x10 != 0 {
                    continue;
                }
                if &e[0..11] == name83 {
                    let cluster = u16::from_le_bytes([e[26], e[27]]);
                    let size = u32::from_le_bytes([e[28], e[29], e[30], e[31]]);
                    buf[off] = 0xE5;
                    if !self.write_sector(self.root_sec + s, &buf) {
                        return None;
                    }
                    return Some((cluster, size));
                }
            }
        }
        None
    }

    /// Free a FAT cluster chain (walks to EOC; multi-cluster after ADR-064).
    fn free_chain(&self, start: u16) -> bool {
        let mut cluster = start;
        while cluster >= 2 && cluster < 0xFFF8 {
            let next = match self.fat_entry(cluster) {
                Some(n) => n,
                None => return false,
            };
            if !self.set_fat_entry(cluster, 0) {
                return false;
            }
            if next >= 0xFFF8 {
                break;
            }
            cluster = next;
        }
        true
    }

    fn find_free_dirent(&self) -> Option<(u32, usize)> {
        let root_secs = (self.root_ents as u32 * 32 + self.bps as u32 - 1) / self.bps as u32;
        let mut buf = [0u8; SECTOR];
        for s in 0..root_secs {
            if !self.read_sector(self.root_sec + s, &mut buf) {
                return None;
            }
            for i in 0..(self.bps as usize / 32) {
                let off = i * 32;
                let first = buf[off];
                if first == 0 || first == 0xE5 {
                    return Some((self.root_sec + s, off));
                }
            }
        }
        None
    }

    fn find_free_cluster(&self) -> Option<u16> {
        let max = self.clusters();
        if max < 2 {
            return None;
        }
        // Cluster IDs are 2..2+max-1.
        for c in 2..(2 + max as u16) {
            match self.fat_entry(c) {
                Some(0) => return Some(c),
                Some(_) => continue,
                None => return None,
            }
        }
        None
    }

    fn read_at(&self, start: u16, size: u32, off: u32, out: &mut [u8]) -> Result<usize, FsError> {
        if off > size {
            return Err(FsError::BadHandle);
        }
        let remain = (size - off) as usize;
        let n = core::cmp::min(out.len(), remain);
        if n == 0 {
            return Ok(0);
        }
        let clus_bytes = self.cluster_bytes();
        let mut cluster = start;
        let mut skipped = 0u32;
        while skipped + clus_bytes <= off {
            cluster = self.fat_entry(cluster).ok_or(FsError::BadHandle)?;
            if cluster < 2 || cluster >= 0xFFF8 {
                return Err(FsError::BadHandle);
            }
            skipped += clus_bytes;
        }
        let mut done = 0;
        let mut cur_off = off - skipped;
        while done < n {
            if cluster < 2 || cluster >= 0xFFF8 {
                break;
            }
            let mut sec = [0u8; SECTOR];
            if !self.read_sector(self.cluster_lba(cluster), &mut sec) {
                return Err(FsError::BadHandle);
            }
            let start = cur_off as usize;
            let take = core::cmp::min(n - done, SECTOR - start);
            out[done..done + take].copy_from_slice(&sec[start..start + take]);
            done += take;
            cur_off = 0;
            if done < n {
                cluster = self.fat_entry(cluster).ok_or(FsError::BadHandle)?;
            }
        }
        Ok(done)
    }

    /// Walk `index` steps from `start` along the FAT chain.
    fn cluster_at(&self, start: u16, index: u32) -> Result<u16, FsError> {
        let mut cluster = start;
        for _ in 0..index {
            cluster = self.fat_entry(cluster).ok_or(FsError::BadHandle)?;
            if cluster < 2 || cluster >= 0xFFF8 {
                return Err(FsError::BadHandle);
            }
        }
        if cluster < 2 {
            return Err(FsError::BadHandle);
        }
        Ok(cluster)
    }

    /// Count clusters in the chain starting at `start` (stops at EOC).
    fn chain_len(&self, start: u16) -> u32 {
        let mut n = 0u32;
        let mut cluster = start;
        while cluster >= 2 && cluster < 0xFFF8 {
            n += 1;
            match self.fat_entry(cluster) {
                Some(next) if next >= 0xFFF8 => return n,
                Some(next) => cluster = next,
                None => return n,
            }
            if n > 64 {
                break;
            }
        }
        n
    }

    /// Ensure the chain from `start` has at least `need` clusters.
    /// Allocates and zeros new clusters; links prior → new → EOC.
    fn extend_chain(&self, start: u16, need: u32) -> Result<(), FsError> {
        if start < 2 || need == 0 || need > 64 {
            return Err(FsError::BadHandle);
        }
        let mut cluster = start;
        let mut have = 1u32;
        loop {
            let next = self.fat_entry(cluster).ok_or(FsError::BadHandle)?;
            if next >= 0xFFF8 {
                break;
            }
            if next < 2 {
                return Err(FsError::BadHandle);
            }
            cluster = next;
            have += 1;
            if have > 64 {
                return Err(FsError::TooBig);
            }
        }
        while have < need {
            let newc = self.find_free_cluster().ok_or(FsError::Full)?;
            if !self.set_fat_entry(newc, EOC) {
                return Err(FsError::BadHandle);
            }
            if !self.set_fat_entry(cluster, newc) {
                let _ = self.set_fat_entry(newc, 0);
                return Err(FsError::BadHandle);
            }
            let zeros = [0u8; SECTOR];
            if !self.write_sector(self.cluster_lba(newc), &zeros) {
                return Err(FsError::BadHandle);
            }
            cluster = newc;
            have += 1;
        }
        Ok(())
    }

    /// Write at `off`, allocating additional FAT clusters when the write
    /// crosses a cluster boundary (ADR-064). Caps at `FILE_MAX`. Assumes
    /// one sector per cluster (`mkfat16` spc=1) this mile.
    fn write_at(&self, start: u16, size: u32, off: u32, data: &[u8]) -> Result<(usize, u32), FsError> {
        if data.is_empty() {
            return Err(FsError::TooBig);
        }
        if off > size {
            return Err(FsError::BadHandle);
        }
        if start < 2 {
            return Err(FsError::BadHandle);
        }
        let clus_bytes = self.cluster_bytes() as usize;
        if clus_bytes == 0 || clus_bytes > SECTOR {
            // First cut: one sector per cluster (mkfat16 uses spc=1).
            return Err(FsError::TooBig);
        }
        let end = off as usize + data.len();
        if end > FILE_MAX {
            return Err(FsError::TooBig);
        }
        let need = ((end + clus_bytes - 1) / clus_bytes) as u32;
        self.extend_chain(start, need)?;

        let mut written = 0usize;
        let mut pos = off as usize;
        while written < data.len() {
            let idx = (pos / clus_bytes) as u32;
            let within = pos % clus_bytes;
            let cluster = self.cluster_at(start, idx)?;
            let mut sec = [0u8; SECTOR];
            if !self.read_sector(self.cluster_lba(cluster), &mut sec) {
                return Err(FsError::BadHandle);
            }
            let take = core::cmp::min(data.len() - written, clus_bytes - within);
            // Zero the gap between old EOF and this write start inside the cluster.
            let old_end = size as usize;
            let cluster_base = idx as usize * clus_bytes;
            let zero_from = if old_end > cluster_base {
                core::cmp::min(old_end - cluster_base, clus_bytes)
            } else {
                0
            };
            if zero_from < within {
                for b in sec[zero_from..within].iter_mut() {
                    *b = 0;
                }
            }
            sec[within..within + take].copy_from_slice(&data[written..written + take]);
            if !self.write_sector(self.cluster_lba(cluster), &sec) {
                return Err(FsError::BadHandle);
            }
            written += take;
            pos += take;
        }
        let new_size = core::cmp::max(size, off + data.len() as u32);
        Ok((data.len(), new_size))
    }


    /// Walk the FAT16 root and fill thin-VFS paths (`/probe`, …).
    /// Skips deleted, volume label, LFN, and subdirectory entries.
    fn list_root(&self, out: &mut [[u8; PATH_MAX]], cap: usize) -> Result<usize, FsError> {
        if !self.mounted {
            return Err(FsError::Missing);
        }
        let cap = core::cmp::min(cap, out.len());
        let root_secs = (self.root_ents as u32 * 32 + self.bps as u32 - 1) / self.bps as u32;
        let mut buf = [0u8; SECTOR];
        let mut n = 0usize;
        for s in 0..root_secs {
            if !self.read_sector(self.root_sec + s, &mut buf) {
                return Err(FsError::BadHandle);
            }
            for i in 0..(self.bps as usize / 32) {
                let e = &buf[i * 32..i * 32 + 32];
                if e[0] == 0 {
                    return Ok(n);
                }
                if e[0] == 0xE5 {
                    continue;
                }
                let attr = e[11];
                if attr & 0x08 != 0 || attr & 0x0F == 0x0F || attr & 0x10 != 0 {
                    continue;
                }
                let mut name83 = [0u8; 11];
                name83.copy_from_slice(&e[0..11]);
                let Some(path) = name83_to_path(&name83) else {
                    continue;
                };
                if n >= cap {
                    return Err(FsError::Full);
                }
                out[n] = path;
                n += 1;
            }
        }
        Ok(n)
    }

    fn alloc_handle(&mut self, name: [u8; 11], cluster: u16, size: u32) -> Result<u32, FsError> {
        for (i, h) in self.handles.iter_mut().enumerate() {
            if !h.used {
                *h = FatHandle {
                    used: true,
                    name,
                    cluster,
                    size,
                    off: 0,
                };
                return Ok((i as u32) + 1);
            }
        }
        Err(FsError::Full)
    }

    fn handle_mut(&mut self, fd: u32) -> Result<&mut FatHandle, FsError> {
        if fd == 0 || fd as usize > MAX_HANDLES {
            return Err(FsError::BadHandle);
        }
        let h = &mut self.handles[(fd as usize) - 1];
        if !h.used {
            return Err(FsError::BadHandle);
        }
        Ok(h)
    }
}

impl VfsOps for Fat16 {
    fn create(&mut self, path: &str) -> Result<u32, FsError> {
        if !self.mounted {
            return Err(FsError::Missing);
        }
        let name = path_to_83(path).ok_or(FsError::BadPath)?;
        if self.lookup(&name).is_some() {
            return Err(FsError::Exists);
        }
        let cluster = self.find_free_cluster().ok_or(FsError::Full)?;
        let (sec, off) = self.find_free_dirent().ok_or(FsError::Full)?;
        if !self.set_fat_entry(cluster, EOC) {
            return Err(FsError::BadHandle);
        }
        let mut buf = [0u8; SECTOR];
        if !self.read_sector(sec, &mut buf) {
            let _ = self.set_fat_entry(cluster, 0);
            return Err(FsError::BadHandle);
        }
        buf[off..off + 11].copy_from_slice(&name);
        buf[off + 11] = ATTR_ARCHIVE;
        for b in buf[off + 12..off + 26].iter_mut() {
            *b = 0;
        }
        let cl = cluster.to_le_bytes();
        buf[off + 26] = cl[0];
        buf[off + 27] = cl[1];
        buf[off + 28..off + 32].copy_from_slice(&0u32.to_le_bytes());
        if !self.write_sector(sec, &buf) {
            let _ = self.set_fat_entry(cluster, 0);
            return Err(FsError::BadHandle);
        }
        // Clear the new cluster so a short read is zeros, not stale.
        let zeros = [0u8; SECTOR];
        if !self.write_sector(self.cluster_lba(cluster), &zeros) {
            return Err(FsError::BadHandle);
        }
        self.alloc_handle(name, cluster, 0)
    }

    fn open(&mut self, path: &str) -> Result<u32, FsError> {
        let name = path_to_83(path).ok_or(FsError::BadPath)?;
        let (cluster, size) = self.lookup(&name).ok_or(FsError::Missing)?;
        self.alloc_handle(name, cluster, size)
    }

    fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
        let (cluster, size, off) = {
            let h = self.handle_mut(fd)?;
            (h.cluster, h.size, h.off)
        };
        let cap = core::cmp::min(buf.len(), FILE_MAX);
        let n = self.read_at(cluster, size, off, &mut buf[..cap])?;
        self.handle_mut(fd)?.off = off + n as u32;
        Ok(n)
    }

    fn write(&mut self, fd: u32, buf: &[u8]) -> Result<usize, FsError> {
        let (name, cluster, size, off) = {
            let h = self.handle_mut(fd)?;
            (h.name, h.cluster, h.size, h.off)
        };
        let (n, new_size) = self.write_at(cluster, size, off, buf)?;
        if new_size != size && !self.update_dirent_size(&name, new_size) {
            return Err(FsError::BadHandle);
        }
        let h = self.handle_mut(fd)?;
        h.size = new_size;
        h.off = off + n as u32;
        Ok(n)
    }

    fn close(&mut self, fd: u32) -> Result<(), FsError> {
        let h = self.handle_mut(fd)?;
        *h = FatHandle::empty();
        Ok(())
    }
}

impl Fat16 {
    /// Delete a root file: mark dirent `0xE5`, free its cluster chain, drop
    /// open handles to that name. Not POSIX `unlink`. Multi-cluster chains ok.
    fn unlink(&mut self, path: &str) -> Result<(), FsError> {
        if !self.mounted {
            return Err(FsError::Missing);
        }
        let name = path_to_83(path).ok_or(FsError::BadPath)?;
        let (cluster, _size) = self.mark_dirent_deleted(&name).ok_or(FsError::Missing)?;
        for h in self.handles.iter_mut() {
            if h.used && h.name == name {
                *h = FatHandle::empty();
            }
        }
        if cluster >= 2 && !self.free_chain(cluster) {
            return Err(FsError::BadHandle);
        }
        Ok(())
    }
}

static FAT: Mutex<Fat16> = Mutex::new(Fat16::empty());
static MOUNT_OK: AtomicBool = AtomicBool::new(false);
static READ_OK: AtomicBool = AtomicBool::new(false);
static WRITE_OK: AtomicBool = AtomicBool::new(false);
static CREATE_OK: AtomicBool = AtomicBool::new(false);
static READDIR_OK: AtomicBool = AtomicBool::new(false);
static DELETE_OK: AtomicBool = AtomicBool::new(false);
static GROW_OK: AtomicBool = AtomicBool::new(false);
static FAT_WRITE_TICKS: AtomicU64 = AtomicU64::new(0);
static MEMFS_WRITE_TICKS: AtomicU64 = AtomicU64::new(0);
static FAT_READ_TICKS: AtomicU64 = AtomicU64::new(0);
static MEMFS_READ_TICKS: AtomicU64 = AtomicU64::new(0);

fn path_to_83(path: &str) -> Option<[u8; 11]> {
    if !vfs::valid_path(path) {
        return None;
    }
    let name = &path.as_bytes()[1..];
    if name.is_empty() || name.len() > 8 {
        return None;
    }
    let mut out = [b' '; 11];
    for (i, &c) in name.iter().enumerate() {
        out[i] = c.to_ascii_uppercase();
    }
    Some(out)
}

/// Map an 8.3 dirent name to a thin-VFS path (`PROBE   ` → `/probe`).
/// Extension must be blank this mile (no `.` in the path grammar).
fn name83_to_path(name83: &[u8; 11]) -> Option<[u8; PATH_MAX]> {
    if name83[8..11] != *b"   " {
        return None;
    }
    let mut len = 0usize;
    while len < 8 && name83[len] != b' ' {
        len += 1;
    }
    if len == 0 || (len < 8 && name83[len..8].iter().any(|&c| c != b' ')) {
        return None;
    }
    let mut path = [0u8; PATH_MAX];
    path[0] = b'/';
    for i in 0..len {
        let c = name83[i].to_ascii_lowercase();
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-') {
            return None;
        }
        path[1 + i] = c;
    }
    let s = core::str::from_utf8(&path[..1 + len]).ok()?;
    if !vfs::valid_path(s) {
        return None;
    }
    Some(path)
}

fn path_buf_str(buf: &[u8; PATH_MAX]) -> &str {
    let mut n = 0usize;
    while n < PATH_MAX && buf[n] != 0 {
        n += 1;
    }
    core::str::from_utf8(&buf[..n]).unwrap_or("")
}

fn mount_from_boot(boot: &[u8; SECTOR]) -> Option<Fat16> {
    if boot[510] != 0x55 || boot[511] != 0xAA {
        return None;
    }
    let bps = u16::from_le_bytes([boot[11], boot[12]]);
    let spc = boot[13];
    let reserved = u16::from_le_bytes([boot[14], boot[15]]);
    let fats = boot[16];
    let root_ents = u16::from_le_bytes([boot[17], boot[18]]);
    let tot16 = u16::from_le_bytes([boot[19], boot[20]]);
    let fat_sz = u16::from_le_bytes([boot[22], boot[23]]);
    let tot32 = u32::from_le_bytes([boot[32], boot[33], boot[34], boot[35]]);
    if bps != 512 || spc == 0 || reserved == 0 || fats == 0 || fat_sz == 0 || root_ents == 0 {
        return None;
    }
    let total = if tot16 != 0 { tot16 as u32 } else { tot32 };
    if total == 0 {
        return None;
    }
    let root_secs = (root_ents as u32 * 32 + bps as u32 - 1) / bps as u32;
    let first_data = reserved as u32 + fats as u32 * fat_sz as u32 + root_secs;
    if first_data >= total {
        return None;
    }
    let clusters = (total - first_data) / spc as u32;
    // Microsoft: 4085..65524 inclusive is FAT16.
    if clusters < 4085 || clusters >= 65525 {
        return None;
    }
    Some(Fat16 {
        mounted: true,
        bps,
        spc,
        reserved,
        fats,
        fat_sz,
        root_ents,
        first_data,
        root_sec: reserved as u32 + fats as u32 * fat_sz as u32,
        total_sec: total,
        handles: [
            FatHandle::empty(),
            FatHandle::empty(),
            FatHandle::empty(),
            FatHandle::empty(),
        ],
    })
}

pub fn init() {
    MOUNT_OK.store(false, Ordering::SeqCst);
    READ_OK.store(false, Ordering::SeqCst);
    WRITE_OK.store(false, Ordering::SeqCst);
    CREATE_OK.store(false, Ordering::SeqCst);
    READDIR_OK.store(false, Ordering::SeqCst);
    DELETE_OK.store(false, Ordering::SeqCst);
    GROW_OK.store(false, Ordering::SeqCst);
    let mut fat = FAT.lock();
    *fat = Fat16::empty();
    if !virtio::ready() {
        return;
    }
    let mut boot = [0u8; SECTOR];
    if !virtio::read_sector(0, &mut boot) {
        return;
    }
    let Some(mounted) = mount_from_boot(&boot) else {
        return;
    };
    *fat = mounted;
    MOUNT_OK.store(true, Ordering::SeqCst);
}

pub fn mounted() -> bool {
    MOUNT_OK.load(Ordering::SeqCst)
}

pub fn has_name(path: &str) -> bool {
    let Some(name) = path_to_83(path) else {
        return false;
    };
    FAT.lock().lookup(&name).is_some()
}

pub fn open(path: &str) -> Result<u32, FsError> {
    FAT.lock().open(path)
}

pub fn create(path: &str) -> Result<u32, FsError> {
    FAT.lock().create(path)
}

pub fn read(fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
    FAT.lock().read(fd, buf)
}

pub fn write(fd: u32, buf: &[u8]) -> Result<usize, FsError> {
    FAT.lock().write(fd, buf)
}

pub fn close(fd: u32) -> Result<(), FsError> {
    FAT.lock().close(fd)
}

/// List FAT16 root file names as thin-VFS paths. Not POSIX `getdents`.
pub fn readdir(out: &mut [[u8; PATH_MAX]], cap: usize) -> Result<usize, FsError> {
    FAT.lock().list_root(out, cap)
}

/// Delete a FAT16 root file behind the thin VFS. Not POSIX `unlink`.
pub fn unlink(path: &str) -> Result<(), FsError> {
    FAT.lock().unlink(path)
}

/// Read a mounted FAT path through the same VFS `open` as A7 `/probe`
/// and A9 `/hello`. Not an embed fallback. Caps at `max` bytes.
pub fn read_file(path: &str, max: usize) -> Option<Vec<u8>> {
    if !mounted() || !has_name(path) {
        return None;
    }
    let Ok(fd) = vfs::open(path) else {
        return None;
    };
    let mut out = Vec::new();
    let mut tmp = [0u8; FILE_MAX];
    loop {
        let Ok(n) = vfs::read(fd, &mut tmp) else {
            let _ = vfs::close(fd);
            return None;
        };
        if n == 0 {
            break;
        }
        if out.len().saturating_add(n) > max {
            let _ = vfs::close(fd);
            return None;
        }
        out.extend_from_slice(&tmp[..n]);
    }
    let _ = vfs::close(fd);
    if out.is_empty() {
        return None;
    }
    Some(out)
}

fn vfs_probe_read() -> bool {
    READ_OK.store(false, Ordering::SeqCst);
    FAT_READ_TICKS.store(0, Ordering::SeqCst);
    let Ok(fd) = vfs::open(PROBE_PATH) else {
        return false;
    };
    let mut buf = [0u8; PATH_MAX];
    // ADR-065: time only the VFS read of the small payload (not open).
    let t0 = timer::cntpct();
    let Ok(n) = vfs::read(fd, &mut buf) else {
        let _ = vfs::close(fd);
        return false;
    };
    let ticks = timer::cntpct().wrapping_sub(t0);
    let _ = vfs::close(fd);
    if n != PROBE_BYTES.len() || &buf[..n] != PROBE_BYTES || ticks == 0 {
        return false;
    }
    FAT_READ_TICKS.store(ticks, Ordering::SeqCst);
    uart::write_str_raw("fat: read\n");
    READ_OK.store(true, Ordering::SeqCst);
    true
}

/// Rewrite `/probe` through the same VFS `write`, read back, then restore.
fn vfs_probe_write() -> bool {
    WRITE_OK.store(false, Ordering::SeqCst);
    FAT_WRITE_TICKS.store(0, Ordering::SeqCst);
    let Ok(fd) = vfs::open(PROBE_PATH) else {
        return false;
    };
    // ADR-051: time only the VFS write of the small payload (not open/restore).
    let t0 = timer::cntpct();
    let Ok(n) = vfs::write(fd, WRITE_BYTES) else {
        let _ = vfs::close(fd);
        return false;
    };
    let ticks = timer::cntpct().wrapping_sub(t0);
    if n != WRITE_BYTES.len() || ticks == 0 {
        let _ = vfs::close(fd);
        return false;
    }
    FAT_WRITE_TICKS.store(ticks, Ordering::SeqCst);
    let _ = vfs::close(fd);

    let Ok(fd) = vfs::open(PROBE_PATH) else {
        return false;
    };
    let mut buf = [0u8; PATH_MAX];
    let Ok(n) = vfs::read(fd, &mut buf) else {
        let _ = vfs::close(fd);
        return false;
    };
    let _ = vfs::close(fd);
    if n != WRITE_BYTES.len() || &buf[..n] != WRITE_BYTES {
        return false;
    }
    uart::write_str_raw("fat: write\n");

    // Restore host-built payload so later `/probe` reads and tests stay stable.
    let Ok(fd) = vfs::open(PROBE_PATH) else {
        return false;
    };
    let Ok(n) = vfs::write(fd, PROBE_BYTES) else {
        let _ = vfs::close(fd);
        return false;
    };
    let _ = vfs::close(fd);
    if n != PROBE_BYTES.len() {
        return false;
    }
    // Dirent size may still be WRITE_BYTES len if that was longer; shrink.
    // write_at grows size with max(old, new_end) and does not shrink. Force
    // size back via a second open path: update by writing exact and patching
    // dirent — handled below.
    if WRITE_BYTES.len() != PROBE_BYTES.len() {
        let name = path_to_83(PROBE_PATH).unwrap();
        if !FAT.lock().update_dirent_size(&name, PROBE_BYTES.len() as u32) {
            return false;
        }
    }
    uart::write_str_raw("fat: rewrite\n");
    WRITE_OK.store(true, Ordering::SeqCst);
    true
}

/// Create `/fwr` on the volume (FAT backend), write bytes, confirm via VFS open.
/// ADR-058 routes `/fwr` to FAT; this probe still calls `fat::create` directly
/// so a re-run can overwrite without depending on vfs create Exists handling.
fn vfs_probe_create() -> bool {
    CREATE_OK.store(false, Ordering::SeqCst);
    if has_name(CREATE_PATH) {
        // Re-run / leftover image: open and overwrite instead of failing Exists.
        let Ok(fd) = open(CREATE_PATH) else {
            return false;
        };
        let Ok(n) = write(fd, CREATE_BYTES) else {
            let _ = close(fd);
            return false;
        };
        let _ = close(fd);
        if n != CREATE_BYTES.len() {
            return false;
        }
    } else {
        let Ok(fd) = create(CREATE_PATH) else {
            return false;
        };
        let Ok(n) = write(fd, CREATE_BYTES) else {
            let _ = close(fd);
            return false;
        };
        let _ = close(fd);
        if n != CREATE_BYTES.len() {
            return false;
        }
    }
    let Ok(fd) = vfs::open(CREATE_PATH) else {
        return false;
    };
    let mut buf = [0u8; PATH_MAX];
    let Ok(n) = vfs::read(fd, &mut buf) else {
        let _ = vfs::close(fd);
        return false;
    };
    let _ = vfs::close(fd);
    if n != CREATE_BYTES.len() || &buf[..n] != CREATE_BYTES {
        return false;
    }
    uart::write_str_raw("fat: create\n");
    CREATE_OK.store(true, Ordering::SeqCst);
    true
}

/// List FAT16 root via the thin VFS entry (`vfs::readdir`). Expect `/probe`,
/// `/hello` (A9 image), `/fsdemo` (ADR-059 when present), `/fatdemo`
/// (ADR-061 when present), and `/fwr` after the create probe. Not POSIX.
fn vfs_probe_readdir() -> bool {
    READDIR_OK.store(false, Ordering::SeqCst);
    let mut names = [[0u8; PATH_MAX]; READDIR_MAX];
    let Ok(n) = vfs::readdir(&mut names, READDIR_MAX) else {
        return false;
    };
    if n == 0 || n > READDIR_MAX {
        return false;
    }
    let mut saw_probe = false;
    let mut saw_hello = false;
    let mut saw_fwr = false;
    for i in 0..n {
        let s = path_buf_str(&names[i]);
        if s == PROBE_PATH {
            saw_probe = true;
        } else if s == "/hello" {
            saw_hello = true;
        } else if s == CREATE_PATH {
            saw_fwr = true;
        }
    }
    if !saw_probe || !saw_hello || !saw_fwr {
        return false;
    }
    uart::write_str_raw("fat: readdir\n");
    let mut w = uart::raw();
    let _ = writeln!(w, "fat: entries n={n}");
    READDIR_OK.store(true, Ordering::SeqCst);
    true
}


/// Create `/fdel`, then delete it via `vfs::unlink`. Confirm Missing + absent
/// from root listing. Keep `/probe`, `/hello`, `/fwr`. Not POSIX `unlink`.
fn vfs_probe_delete() -> bool {
    DELETE_OK.store(false, Ordering::SeqCst);
    // Leftover image: clear any prior /fdel first.
    if has_name(DELETE_PATH) {
        if unlink(DELETE_PATH).is_err() {
            return false;
        }
    }
    let Ok(fd) = create(DELETE_PATH) else {
        return false;
    };
    let Ok(n) = write(fd, DELETE_BYTES) else {
        let _ = close(fd);
        return false;
    };
    let _ = close(fd);
    if n != DELETE_BYTES.len() || !has_name(DELETE_PATH) {
        return false;
    }
    if vfs::unlink(DELETE_PATH).is_err() {
        return false;
    }
    if has_name(DELETE_PATH) {
        return false;
    }
    if open(DELETE_PATH) != Err(FsError::Missing) {
        return false;
    }
    // Root listing must keep write-mile + A9 names and must not list /fdel.
    let mut names = [[0u8; PATH_MAX]; READDIR_MAX];
    let Ok(n) = vfs::readdir(&mut names, READDIR_MAX) else {
        return false;
    };
    let mut saw_probe = false;
    let mut saw_hello = false;
    let mut saw_fwr = false;
    for i in 0..n {
        let s = path_buf_str(&names[i]);
        if s == DELETE_PATH {
            return false;
        } else if s == PROBE_PATH {
            saw_probe = true;
        } else if s == "/hello" {
            saw_hello = true;
        } else if s == CREATE_PATH {
            saw_fwr = true;
        }
    }
    if !saw_probe || !saw_hello || !saw_fwr {
        return false;
    }
    uart::write_str_raw("fat: delete\n");
    DELETE_OK.store(true, Ordering::SeqCst);
    true
}


/// Create `/fgrow`, write `GROW_LEN` (> one cluster) bytes across a cluster
/// boundary, confirm chain length ≥ 2 and VFS read-back (ADR-064).
fn vfs_probe_grow() -> bool {
    GROW_OK.store(false, Ordering::SeqCst);
    if has_name(GROW_PATH) {
        if unlink(GROW_PATH).is_err() {
            return false;
        }
    }
    let Ok(fd) = create(GROW_PATH) else {
        return false;
    };
    let mut payload = [0u8; GROW_LEN];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = b'A'.wrapping_add((i % 26) as u8);
    }
    let Ok(n) = write(fd, &payload) else {
        let _ = close(fd);
        return false;
    };
    let _ = close(fd);
    if n != GROW_LEN {
        return false;
    }
    let Some(name) = path_to_83(GROW_PATH) else {
        return false;
    };
    let (cluster, size, clen) = {
        let fat = FAT.lock();
        let Some((c, s)) = fat.lookup(&name) else {
            return false;
        };
        (c, s, fat.chain_len(c))
    };
    if size != GROW_LEN as u32 || clen < 2 || cluster < 2 {
        return false;
    }
    let Ok(fd) = vfs::open(GROW_PATH) else {
        return false;
    };
    let mut buf = [0u8; GROW_LEN];
    let Ok(n) = vfs::read(fd, &mut buf) else {
        let _ = vfs::close(fd);
        return false;
    };
    let _ = vfs::close(fd);
    if n != GROW_LEN || buf != payload {
        return false;
    }
    // Boundary bytes must differ across the cluster edge (spc=1 → 512).
    let edge = 512usize;
    if edge >= GROW_LEN || buf[edge - 1] == buf[edge] {
        // Pattern A..Z makes offset 511 and 512 different (511%26 != 512%26).
        return false;
    }
    uart::write_str_raw("fat: grow\n");
    GROW_OK.store(true, Ordering::SeqCst);
    true
}

/// Memfs read of the same small payload as FAT `/probe` (ADR-065).
/// Seeds `/mrprobe` then times only `vfs::read`, not create/open/write. Not a bench.
fn measure_memfs_read() -> Option<u64> {
    MEMFS_READ_TICKS.store(0, Ordering::SeqCst);
    let fd = match vfs::create(MEMFS_READ_CMP_PATH) {
        Ok(fd) => fd,
        Err(FsError::Exists) => match vfs::open(MEMFS_READ_CMP_PATH) {
            Ok(fd) => fd,
            Err(_) => return None,
        },
        Err(_) => return None,
    };
    // Seed outside the timed window (same payload as FAT `/probe`).
    let n = match vfs::write(fd, PROBE_BYTES) {
        Ok(n) => n,
        Err(_) => {
            let _ = vfs::close(fd);
            return None;
        }
    };
    let _ = vfs::close(fd);
    if n != PROBE_BYTES.len() {
        return None;
    }
    let Ok(fd) = vfs::open(MEMFS_READ_CMP_PATH) else {
        return None;
    };
    let mut buf = [0u8; PATH_MAX];
    let t0 = timer::cntpct();
    let n = match vfs::read(fd, &mut buf) {
        Ok(n) => n,
        Err(_) => {
            let _ = vfs::close(fd);
            return None;
        }
    };
    let ticks = timer::cntpct().wrapping_sub(t0);
    let _ = vfs::close(fd);
    if n != PROBE_BYTES.len() || &buf[..n] != PROBE_BYTES || ticks == 0 {
        return None;
    }
    MEMFS_READ_TICKS.store(ticks, Ordering::SeqCst);
    Some(ticks)
}

/// Memfs write of the same small payload as the FAT `/probe` rewrite (ADR-051).
/// Times only `vfs::write`, not create/open. Not a bench.
fn measure_memfs_write() -> Option<u64> {
    MEMFS_WRITE_TICKS.store(0, Ordering::SeqCst);
    let fd = match vfs::create(MEMFS_CMP_PATH) {
        Ok(fd) => fd,
        Err(FsError::Exists) => match vfs::open(MEMFS_CMP_PATH) {
            Ok(fd) => fd,
            Err(_) => return None,
        },
        Err(_) => return None,
    };
    let t0 = timer::cntpct();
    let n = match vfs::write(fd, WRITE_BYTES) {
        Ok(n) => n,
        Err(_) => {
            let _ = vfs::close(fd);
            return None;
        }
    };
    let ticks = timer::cntpct().wrapping_sub(t0);
    let _ = vfs::close(fd);
    if n != WRITE_BYTES.len() || ticks == 0 {
        return None;
    }
    MEMFS_WRITE_TICKS.store(ticks, Ordering::SeqCst);
    Some(ticks)
}

/// Serial proof: mount, VFS-read `/probe`, VFS-write + restore, create `/fwr`,
/// root `readdir` (ADR-056), delete `/fdel` (ADR-057), multi-cluster grow
/// `/fgrow` (ADR-064), plus ADR-051 write and ADR-065 read FAT-vs-memfs
/// CNTPCT pairs (raw ticks).
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    if !mounted() {
        return false;
    }
    uart::write_str_raw("fat: mount\n");
    if !vfs_probe_read() {
        return false;
    }
    if !vfs_probe_write() {
        return false;
    }
    if !vfs_probe_create() {
        return false;
    }
    if !vfs_probe_readdir() {
        return false;
    }
    if !vfs_probe_delete() {
        return false;
    }
    if !vfs_probe_grow() {
        return false;
    }
    let mut w = uart::raw();
    let fat_read_ticks = FAT_READ_TICKS.load(Ordering::SeqCst);
    if fat_read_ticks == 0 {
        let _ = writeln!(w, "perf: fat-read missed");
        return false;
    }
    let _ = writeln!(w, "perf: fat-read ticks={fat_read_ticks}");
    let Some(memfs_read_ticks) = measure_memfs_read() else {
        let _ = writeln!(w, "perf: memfs-read missed");
        return false;
    };
    let _ = writeln!(w, "perf: memfs-read ticks={memfs_read_ticks}");
    let _ = writeln!(
        w,
        "perf: fs-read-delta fat={fat_read_ticks} memfs={memfs_read_ticks}"
    );
    let fat_ticks = FAT_WRITE_TICKS.load(Ordering::SeqCst);
    if fat_ticks == 0 {
        let _ = writeln!(w, "perf: fat-write missed");
        return false;
    }
    let _ = writeln!(w, "perf: fat-write ticks={fat_ticks}");
    let Some(memfs_ticks) = measure_memfs_write() else {
        let _ = writeln!(w, "perf: memfs-write missed");
        return false;
    };
    let _ = writeln!(w, "perf: memfs-write ticks={memfs_ticks}");
    let _ = writeln!(w, "perf: fs-write-delta fat={fat_ticks} memfs={memfs_ticks}");
    let _ = writeln!(w, "fat: ok");
    true
}

#[cfg(test)]
#[test_case]
fn fat16_vfs_open_probe() {
    assert!(mounted(), "FAT16 must mount from the attached image");
    assert!(has_name(PROBE_PATH));
    assert!(vfs_probe_read());
    assert!(READ_OK.load(Ordering::SeqCst));
}

#[cfg(test)]
#[test_case]
fn fat16_vfs_write_and_restore() {
    assert!(mounted());
    assert!(
        vfs_probe_write(),
        "VFS write on /probe must round-trip and restore fat-hi"
    );
    assert!(WRITE_OK.load(Ordering::SeqCst));
    assert!(vfs_probe_read());
}

#[cfg(test)]
#[test_case]
fn fat16_create_write_small_file() {
    assert!(mounted());
    assert!(
        vfs_probe_create(),
        "FAT create+/fwr write must be VFS-readable"
    );
    assert!(CREATE_OK.load(Ordering::SeqCst));
    assert!(has_name(CREATE_PATH));
}

#[cfg(test)]
#[test_case]
fn fat_memfs_read_cntpct_pair() {
    assert!(mounted(), "FAT16 must mount for the read CNTPCT pair");
    assert!(
        vfs_probe_read(),
        "FAT VFS read must round-trip and store fat-read ticks"
    );
    let fat_ticks = FAT_READ_TICKS.load(Ordering::SeqCst);
    assert!(fat_ticks > 0, "perf: fat-read sample must be > 0");
    let memfs_ticks = measure_memfs_read().expect("memfs read CNTPCT must advance");
    assert!(memfs_ticks > 0, "perf: memfs-read sample must be > 0");
    assert_eq!(
        MEMFS_READ_TICKS.load(Ordering::SeqCst),
        memfs_ticks,
        "pair prints raw fat=<a> memfs=<b> ticks"
    );
}

#[cfg(test)]
#[test_case]
fn fat_memfs_write_cntpct_pair() {
    assert!(mounted(), "FAT16 must mount for the write CNTPCT pair");
    assert!(
        vfs_probe_write(),
        "FAT VFS write must round-trip and store fat-write ticks"
    );
    let fat_ticks = FAT_WRITE_TICKS.load(Ordering::SeqCst);
    assert!(fat_ticks > 0, "perf: fat-write sample must be > 0");
    let memfs_ticks = measure_memfs_write().expect("memfs write CNTPCT must advance");
    assert!(memfs_ticks > 0, "perf: memfs-write sample must be > 0");
    assert_eq!(
        MEMFS_WRITE_TICKS.load(Ordering::SeqCst),
        memfs_ticks,
        "pair prints raw fat=<a> memfs=<b> ticks"
    );
}

#[cfg(test)]
#[test_case]
fn fat16_missing_and_limits() {
    assert!(mounted());
    assert_eq!(open("/missing"), Err(FsError::Missing));
    assert_eq!(open("/probe-too-long-name"), Err(FsError::BadPath));
    let fd = open(PROBE_PATH).expect("PROBE");
    // Empty write is rejected (same TooBig shape as memfs).
    assert_eq!(write(fd, b""), Err(FsError::TooBig));
    assert_eq!(close(fd), Ok(()));
    // Creating an existing FAT name fails.
    assert_eq!(create(PROBE_PATH), Err(FsError::Exists));
}


#[cfg(test)]
#[test_case]
fn fat16_readdir_lists_root() {
    assert!(mounted(), "FAT16 must mount for readdir");
    assert!(
        vfs_probe_create(),
        "create /fwr so readdir sees the write-mile name"
    );
    assert!(
        vfs_probe_readdir(),
        "root readdir must list /probe, /hello, /fwr"
    );
    assert!(READDIR_OK.load(Ordering::SeqCst));
    let mut names = [[0u8; PATH_MAX]; READDIR_MAX];
    let n = readdir(&mut names, READDIR_MAX).expect("readdir");
    assert!(n >= 3);
    let mut paths = [false; 3];
    for i in 0..n {
        match path_buf_str(&names[i]) {
            "/probe" => paths[0] = true,
            "/hello" => paths[1] = true,
            "/fwr" => paths[2] = true,
            _ => {}
        }
    }
    assert!(paths[0] && paths[1] && paths[2]);
}

#[cfg(test)]
#[test_case]
fn fat16_readdir_fail_closed_cap() {
    assert!(mounted());
    let mut names = [[0u8; PATH_MAX]; 1];
    // Cap of 1 with >=2 root files must return Full (fail-closed).
    assert_eq!(readdir(&mut names, 1), Err(FsError::Full));
}

#[cfg(test)]
#[test_case]
fn fat16_path_maps_8_3() {
    assert_eq!(path_to_83("/probe"), Some(*b"PROBE      "));
    assert_eq!(path_to_83("/hello"), Some(*b"HELLO      "));
    assert_eq!(path_to_83("/kprobe"), Some(*b"KPROBE     "));
    assert_eq!(path_to_83("/fwr"), Some(*b"FWR        "));
    assert_eq!(path_to_83("/fsdemo"), Some(*b"FSDEMO     "));
    assert!(path_to_83("/toolong12").is_none());
    assert!(path_to_83("probe").is_none());
    assert_eq!(
        path_buf_str(&name83_to_path(b"PROBE      ").unwrap()),
        "/probe"
    );
    assert_eq!(
        path_buf_str(&name83_to_path(b"HELLO      ").unwrap()),
        "/hello"
    );
    assert_eq!(
        path_buf_str(&name83_to_path(b"FWR        ").unwrap()),
        "/fwr"
    );
    assert!(name83_to_path(b"PROBE   TXT").is_none());
}

#[cfg(test)]
#[test_case]
fn fat16_unlink_removes_file() {
    assert!(mounted(), "FAT16 must mount for unlink");
    assert!(
        vfs_probe_delete(),
        "create+/fdel then vfs::unlink must leave Missing and keep /fwr"
    );
    assert!(DELETE_OK.load(Ordering::SeqCst));
    assert!(!has_name(DELETE_PATH));
    assert_eq!(open(DELETE_PATH), Err(FsError::Missing));
    assert!(has_name(PROBE_PATH));
    assert!(has_name(CREATE_PATH));
    assert!(has_name("/hello"));
}

#[cfg(test)]
#[test_case]
fn fat16_unlink_missing_is_err() {
    assert!(mounted());
    assert_eq!(unlink("/missing"), Err(FsError::Missing));
    assert_eq!(unlink("bad"), Err(FsError::BadPath));
    assert_eq!(vfs::unlink("/missing"), Err(FsError::Missing));
}

#[cfg(test)]
#[test_case]
fn fat16_multi_cluster_grow() {
    assert!(mounted(), "FAT16 must mount for multi-cluster grow");
    assert!(
        vfs_probe_grow(),
        "FAT create+/fgrow must allocate ≥2 clusters and read back"
    );
    assert!(GROW_OK.load(Ordering::SeqCst));
    assert!(has_name(GROW_PATH));
    let name = path_to_83(GROW_PATH).expect("fgrow 8.3");
    let fat = FAT.lock();
    let (c, s) = fat.lookup(&name).expect("fgrow dirent");
    assert_eq!(s, GROW_LEN as u32);
    assert!(fat.chain_len(c) >= 2);
}

#[cfg(test)]
#[test_case]
fn fat16_grow_refuses_over_file_max() {
    assert!(mounted());
    const PATH: &str = "/fbig";
    if has_name(PATH) {
        assert_eq!(unlink(PATH), Ok(()));
    }
    let fd = create(PATH).expect("create fbig");
    let big = [b'x'; FILE_MAX + 1];
    assert_eq!(write(fd, &big), Err(FsError::TooBig));
    assert_eq!(close(fd), Ok(()));
    let _ = unlink(PATH);
}

