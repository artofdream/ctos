# Session memory — 2026-09-12 A7 virtio-blk + FAT16

## Branch / PR

`cursor/a7-virtio-blk-fat-a587` → https://github.com/artofdream/ctos/pull/57  
Base: `main` `9878e70` (A6 / ADR-027).

## Format choice

FAT16, not xv6-like: host `scripts/mkfat16.py` can build and `--check` the raw image. FAT32 rejected (root-as-cluster-chain). Cluster count 8095 so the volume is FAT16, not FAT12.

## What broke first

Hello printed `blk: probe missed`. QEMU 8.2 `virt` virtio-mmio transports are **legacy version 1** (`QueuePFN` + page-aligned desc/avail/used). Modern-only scan skipped the device. Fixed by accepting v1 and laying DMA as page 0 = desc+avail, page 1 = used.

## Probe (this cloud VM)

QEMU 8.2.2, rustc 1.100.0-nightly `0fc141305`, `-cpu cortex-a57`.  
`./scripts/qemu-smoke.sh` → `qemu-smoke: ok`.  
Hello: `blk: cap sectors=8192` then `fat: ok`.  
`cargo test` 79 cases. force-fail exit 1.

QEMU flags: `-drive if=none,file=target/fat16.img,format=raw,id=hd0 -device virtio-blk-device,drive=hd0`

## A8 pickup

Documented sample apps (#39): payload that `fs_open("/probe")` / `fs_read` (or a memfs name) via existing SVC 19–23. Not a second FS. Not POSIX.
