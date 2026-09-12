#!/usr/bin/env python3
"""Build a 4 MiB FAT16 raw image.

Always writes 8.3 PROBE = b'fat-hi' (VFS /probe, A7).
With --app (or CTOS_APP_ELF / target/hello-libctos.elf) also writes
8.3 HELLO = the app ELF (VFS /hello, A9 / ADR-030).

Host-visible: the same bytes the guest opens. Cluster count is >= 4085
so the volume is FAT16, not FAT12. Not a guest probe. Not FAT32.
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
HELLO_NAME = b"HELLO   " + b"   "
HELLO_CLUSTER = 3
APP_MAX = 64 * 1024
EOC = 0xFFFF


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


def cluster_bytes() -> int:
    return SEC_PER_CLUS * BYTES_PER_SEC


def clusters_for(size: int) -> int:
    if size <= 0:
        return 1
    cb = cluster_bytes()
    return (size + cb - 1) // cb


def default_app_path() -> str | None:
    env = os.environ.get("CTOS_APP_ELF")
    if env:
        return env
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    cand = os.path.join(root, "target", "hello-libctos.elf")
    if os.path.isfile(cand):
        return cand
    return None


def _write_fat_entry(fat: bytearray, cluster: int, value: int) -> None:
    off = cluster * 2
    fat[off : off + 2] = _u16(value)


def _write_chain(fat: bytearray, start: int, nclus: int) -> None:
    for i in range(nclus):
        cl = start + i
        nxt = EOC if i == nclus - 1 else cl + 1
        _write_fat_entry(fat, cl, nxt)


def _put_dirent(img: bytearray, root_off: int, index: int, name: bytes, cluster: int, size: int) -> None:
    ent = bytearray(32)
    ent[0:11] = name
    ent[11] = 0x20
    ent[26:28] = _u16(cluster)
    ent[28:32] = _u32(size)
    off = root_off + index * 32
    img[off : off + 32] = ent


def _put_data(img: bytearray, start_cluster: int, data: bytes) -> None:
    data_off = first_data_sector() * BYTES_PER_SEC + (start_cluster - 2) * cluster_bytes()
    img[data_off : data_off + len(data)] = data


def build(app: bytes | None) -> bytearray:
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
    _write_fat_entry(fat, 0, 0xFF00 | MEDIA)
    _write_fat_entry(fat, 1, EOC)
    _write_chain(fat, PROBE_CLUSTER, 1)

    root = (first_data_sector() - (ROOT_ENTS * 32) // BYTES_PER_SEC) * BYTES_PER_SEC
    _put_dirent(img, root, 0, PROBE_NAME, PROBE_CLUSTER, len(PROBE_BYTES))
    _put_data(img, PROBE_CLUSTER, PROBE_BYTES)

    if app is not None:
        if not app:
            raise SystemExit("mkfat16: --app is empty")
        if len(app) > APP_MAX:
            raise SystemExit(f"mkfat16: app {len(app)} bytes is > {APP_MAX}")
        if app[0:4] != b"\x7fELF":
            raise SystemExit("mkfat16: --app is not an ELF")
        nclus = clusters_for(len(app))
        last = HELLO_CLUSTER + nclus - 1
        if last >= cluster_count() + 2:
            raise SystemExit(f"mkfat16: app needs {nclus} clusters; volume too small")
        _write_chain(fat, HELLO_CLUSTER, nclus)
        _put_dirent(img, root, 1, HELLO_NAME, HELLO_CLUSTER, len(app))
        _put_data(img, HELLO_CLUSTER, app)

    for i in range(NUM_FATS):
        off = (RESERVED + i * FAT_SZ) * BYTES_PER_SEC
        img[off : off + len(fat)] = fat
    return img


def _dirent_at(img: bytes, root: int, index: int) -> bytes:
    off = root + index * 32
    return img[off : off + 32]


def _cluster_data(img: bytes, cluster: int, size: int) -> bytes:
    off = first_data_sector() * BYTES_PER_SEC + (cluster - 2) * cluster_bytes()
    return img[off : off + size]


def check(path: str, require_app: bool) -> None:
    with open(path, "rb") as f:
        img = f.read()
    if len(img) != TOTAL_SEC * BYTES_PER_SEC:
        raise SystemExit(f"mkfat16: bad size {len(img)}")
    if img[510:512] != b"\x55\xaa":
        raise SystemExit("mkfat16: missing 0x55AA")
    if img[54:62] != b"FAT16   ":
        raise SystemExit("mkfat16: FS type is not FAT16")
    root = (first_data_sector() - (ROOT_ENTS * 32) // BYTES_PER_SEC) * BYTES_PER_SEC
    probe = _dirent_at(img, root, 0)
    if probe[0:11] != PROBE_NAME:
        raise SystemExit("mkfat16: PROBE dirent missing")
    if _cluster_data(img, PROBE_CLUSTER, len(PROBE_BYTES)) != PROBE_BYTES:
        raise SystemExit("mkfat16: PROBE bytes missing")
    hello = _dirent_at(img, root, 1)
    has_hello = hello[0:11] == HELLO_NAME
    if require_app and not has_hello:
        raise SystemExit("mkfat16: HELLO dirent missing (--require-app)")
    extra = ""
    if has_hello:
        size = int.from_bytes(hello[28:32], "little")
        if size < 64 or size > APP_MAX:
            raise SystemExit(f"mkfat16: HELLO size {size} is implausible")
        data = _cluster_data(img, HELLO_CLUSTER, size)
        if data[0:4] != b"\x7fELF":
            raise SystemExit("mkfat16: HELLO is not an ELF")
        extra = f" file=/hello bytes={size}"
    print(
        f"mkfat16: check ok file=/probe bytes={PROBE_BYTES.decode()}{extra} "
        f"sectors={TOTAL_SEC} clusters={cluster_count()}"
    )


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("image", help="raw FAT16 image path")
    p.add_argument(
        "--app",
        metavar="ELF",
        help="app ELF to store as FAT /hello (A9). Default: CTOS_APP_ELF or target/hello-libctos.elf",
    )
    p.add_argument(
        "--no-app",
        action="store_true",
        help="write PROBE only (A7-only image; A9 guest path will miss)",
    )
    p.add_argument(
        "--check",
        action="store_true",
        help="verify an existing image (host-visible, not a guest probe)",
    )
    p.add_argument(
        "--require-app",
        action="store_true",
        help="with --check, require FAT /hello ELF",
    )
    args = p.parse_args()
    if args.check:
        check(args.image, require_app=args.require_app)
        return 0

    app_bytes = None
    if not args.no_app:
        app_path = args.app or default_app_path()
        if app_path is None:
            if args.app:
                raise SystemExit("mkfat16: --app path missing")
        else:
            if not os.path.isfile(app_path):
                raise SystemExit(f"mkfat16: app ELF not found: {app_path}")
            with open(app_path, "rb") as f:
                app_bytes = f.read()

    os.makedirs(os.path.dirname(os.path.abspath(args.image)) or ".", exist_ok=True)
    img = build(app_bytes)
    with open(args.image, "wb") as f:
        f.write(img)
    extra = ""
    if app_bytes is not None:
        extra = f" file=/hello bytes={len(app_bytes)}"
    print(
        f"mkfat16: wrote {args.image} bytes={len(img)} "
        f"file=/probe payload={PROBE_BYTES.decode()}{extra} "
        f"clusters={cluster_count()}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
