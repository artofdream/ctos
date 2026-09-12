#!/usr/bin/env python3
"""Build a 4 MiB FAT16 raw image with one 8.3 file PROBE = b'fat-hi'.

Host-visible: the same bytes the guest opens as VFS path /probe.
Cluster count is >= 4085 so the volume is FAT16, not FAT12.
Not a guest probe. Not FAT32.
"""
from __future__ import annotations

import argparse
import os
import sys

BYTES_PER_SEC = 512
SEC_PER_CLUS = 1
RESERVED = 1
NUM_FATS = 2
ROOT_ENTS = 512
FAT_SZ = 32
TOTAL_SEC = 8192  # 4 MiB
MEDIA = 0xF8
PROBE_NAME = b"PROBE   " + b"   "  # 8.3, no extension
PROBE_BYTES = b"fat-hi"
PROBE_CLUSTER = 2


def _u16(n: int) -> bytes:
    return n.to_bytes(2, "little")


def _u32(n: int) -> bytes:
    return n.to_bytes(4, "little")


def first_data_sector() -> int:
    root_secs = (ROOT_ENTS * 32 + BYTES_PER_SEC - 1) // BYTES_PER_SEC
    return RESERVED + NUM_FATS * FAT_SZ + root_secs


def cluster_count() -> int:
    data = TOTAL_SEC - first_data_sector()
    return data // SEC_PER_CLUS


def build() -> bytearray:
    clusters = cluster_count()
    if clusters < 4085 or clusters >= 65525:
        raise SystemExit(f"mkfat16: cluster count {clusters} is not FAT16")
    img = bytearray(TOTAL_SEC * BYTES_PER_SEC)

    bpb = bytearray(BYTES_PER_SEC)
    bpb[0:3] = b"\xeb\x3c\x90"
    bpb[3:11] = b"CTOSFAT1"
    bpb[11:13] = _u16(BYTES_PER_SEC)
    bpb[13] = SEC_PER_CLUS
    bpb[14:16] = _u16(RESERVED)
    bpb[16] = NUM_FATS
    bpb[17:19] = _u16(ROOT_ENTS)
    bpb[19:21] = _u16(0)  # total_sec_16; use 32-bit
    bpb[21] = MEDIA
    bpb[22:24] = _u16(FAT_SZ)
    bpb[24:26] = _u16(32)
    bpb[26:28] = _u16(2)
    bpb[32:36] = _u32(TOTAL_SEC)
    bpb[36] = 0x80
    bpb[38] = 0x29
    bpb[39:43] = b"CTOS"
    bpb[43:54] = b"CTOSFAT16  "
    bpb[54:62] = b"FAT16   "
    bpb[510:512] = b"\x55\xaa"
    img[0:BYTES_PER_SEC] = bpb

    fat = bytearray(FAT_SZ * BYTES_PER_SEC)
    fat[0:2] = _u16(0xFF00 | MEDIA)
    fat[2:4] = _u16(0xFFFF)
    fat[4:6] = _u16(0xFFFF)  # cluster 2 = EOC
    for i in range(NUM_FATS):
        off = (RESERVED + i * FAT_SZ) * BYTES_PER_SEC
        img[off : off + len(fat)] = fat

    root = first_data_sector() - (ROOT_ENTS * 32) // BYTES_PER_SEC
    ent = bytearray(32)
    ent[0:11] = PROBE_NAME
    ent[11] = 0x20
    ent[26:28] = _u16(PROBE_CLUSTER)
    ent[28:32] = _u32(len(PROBE_BYTES))
    img[root * BYTES_PER_SEC : root * BYTES_PER_SEC + 32] = ent

    data = first_data_sector() * BYTES_PER_SEC
    img[data : data + len(PROBE_BYTES)] = PROBE_BYTES
    return img


def check(path: str) -> None:
    with open(path, "rb") as f:
        img = f.read()
    if len(img) != TOTAL_SEC * BYTES_PER_SEC:
        raise SystemExit(f"mkfat16: bad size {len(img)}")
    if img[510:512] != b"\x55\xaa":
        raise SystemExit("mkfat16: missing 0x55AA")
    if img[54:62] != b"FAT16   ":
        raise SystemExit("mkfat16: FS type is not FAT16")
    root = (first_data_sector() - (ROOT_ENTS * 32) // BYTES_PER_SEC) * BYTES_PER_SEC
    if img[root : root + 11] != PROBE_NAME:
        raise SystemExit("mkfat16: PROBE dirent missing")
    data = first_data_sector() * BYTES_PER_SEC
    if img[data : data + len(PROBE_BYTES)] != PROBE_BYTES:
        raise SystemExit("mkfat16: PROBE bytes missing")
    print(
        f"mkfat16: check ok file=/probe bytes={PROBE_BYTES.decode()} "
        f"sectors={TOTAL_SEC} clusters={cluster_count()}"
    )


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("image", help="raw FAT16 image path")
    p.add_argument(
        "--check",
        action="store_true",
        help="verify an existing image (host-visible, not a guest probe)",
    )
    args = p.parse_args()
    if args.check:
        check(args.image)
        return 0
    os.makedirs(os.path.dirname(os.path.abspath(args.image)) or ".", exist_ok=True)
    img = build()
    with open(args.image, "wb") as f:
        f.write(img)
    print(
        f"mkfat16: wrote {args.image} bytes={len(img)} "
        f"file=/probe payload={PROBE_BYTES.decode()} clusters={cluster_count()}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
