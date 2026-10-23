#!/bin/sh
# ADR-107: host FAT32 test image for the second virtio-blk disk.
# Built by the real tool (mkfs.fat -F 32 + mtools), not a hand-rolled writer.
#   mkfat32.sh OUT.img            build (fragmented BIG.BIN; root spans >1 cluster)
#   mkfat32.sh --corrupt IN OUT   copy IN and break the root chain (negative)
# Host image alone is not a guest probe; fat32: ok on serial is.
set -eu
need() { command -v "$1" >/dev/null 2>&1 || { echo "mkfat32: missing $1 (install dosfstools + mtools)" >&2; exit 1; }; }
need mkfs.fat; need mcopy; need mdel; need python3
export MTOOLS_SKIP_CHECK=1

if [ "${1:-}" = "--corrupt" ]; then
    in=$2; out=$3
    cp "$in" "$out"
    # Root chain: set FAT[root] to 0 (free) in every FAT copy. The guest must
    # report fat32: fail reason=bad-chain, never fat32: ok.
    python3 - "$out" <<'PY'
import struct, sys
p = sys.argv[1]
with open(p, "r+b") as f:
    b = f.read(512)
    res = struct.unpack_from("<H", b, 14)[0]
    nfats = b[16]
    fatsz = struct.unpack_from("<I", b, 36)[0]
    root = struct.unpack_from("<I", b, 44)[0]
    for k in range(nfats):
        f.seek((res + k * fatsz) * 512 + root * 4)
        f.write(struct.pack("<I", 0))
PY
    exit 0
fi

out=${1:?usage: mkfat32.sh OUT.img}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
rm -f "$out"
# 34 MiB with 512-byte clusters: about 68k clusters, above the 65525 FAT32 floor.
mkfs.fat -F 32 -s 1 -S 512 -n CTOSF32 -C "$out" 34816 >/dev/null
printf 'fat32-hi\n' > "$tmp/PROBE32.TXT"
python3 -c "import sys; sys.stdout.buffer.write(bytes((i*7+3)%251 for i in range(3000)))" > "$tmp/BIG.BIN"
mcopy -i "$out" "$tmp/PROBE32.TXT" ::PROBE32.TXT
mcopy -i "$out" "$tmp/BIG.BIN" ::BIG.BIN
# 20 more 8.3 entries push the root directory past one 512-byte cluster.
i=0
while [ $i -lt 20 ]; do
    n=$(printf 'F%02d.TXT' $i)
    printf 'f%02d\n' $i > "$tmp/$n"
    mcopy -i "$out" "$tmp/$n" "::$n"
    i=$((i + 1))
done
printf 'fat32-root-chain\n' > "$tmp/LAST.TXT"
mcopy -i "$out" "$tmp/LAST.TXT" ::LAST.TXT
# mtools allocates forward, so BIG.BIN is contiguous. Move its last three
# clusters far up the volume (a free run) and relink them in every FAT copy:
# a valid but fragmented FAT32 chain, so the guest must follow real links.
python3 - "$out" <<'PY'
import struct, sys
p = sys.argv[1]
with open(p, "r+b") as f:
    img = bytearray(f.read())
    res = struct.unpack_from("<H", img, 14)[0]
    nfats = img[16]
    fatsz = struct.unpack_from("<I", img, 36)[0]
    spc = img[13]
    first_data = res + nfats * fatsz
    def fat_get(c):
        return struct.unpack_from("<I", img, res * 512 + c * 4)[0] & 0x0FFFFFFF
    def fat_set(c, v):
        for k in range(nfats):
            struct.pack_into("<I", img, (res + k * fatsz) * 512 + c * 4, v)
    def lba(c):
        return (first_data + (c - 2) * spc) * 512
    # Find BIG.BIN in the root cluster (8.3 entry).
    root = struct.unpack_from("<I", img, 44)[0]
    off = lba(root)
    start = None
    for i in range(0, 512 * spc, 32):
        if bytes(img[off + i:off + i + 11]) == b"BIG     BIN":
            e = off + i
            start = (struct.unpack_from("<H", img, e + 20)[0] << 16) | struct.unpack_from("<H", img, e + 26)[0]
    assert start, "BIG.BIN not in first root cluster"
    chain = [start]
    while fat_get(chain[-1]) < 0x0FFFFFF8:
        chain.append(fat_get(chain[-1]))
    assert len(chain) >= 4, chain
    dest = 40000
    moved = chain[-3:]
    for j, c in enumerate(moved):
        assert fat_get(dest + j) == 0
    for j, c in enumerate(moved):
        n = dest + j
        img[lba(n):lba(n) + 512 * spc] = img[lba(c):lba(c) + 512 * spc]
        img[lba(c):lba(c) + 512 * spc] = bytes(512 * spc)
        fat_set(c, 0)
    fat_set(chain[-4], dest)
    fat_set(dest, dest + 1)
    fat_set(dest + 1, dest + 2)
    fat_set(dest + 2, 0x0FFFFFFF)
    f.seek(0)
    f.write(img)
PY
# Host check only (not a guest probe): the moved chain is still a clean FS.
fsck.fat -n "$out" >/dev/null
