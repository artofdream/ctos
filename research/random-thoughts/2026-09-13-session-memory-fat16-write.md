# Session memory — FAT16 write

- Preferred H1 over H2 (readdir). Write fit one PR.
- `vfs::create` must stay memfs-first: FAT mounted during A6 would otherwise park `/kprobe` on disk.
- Probe restores `/probe` to `fat-hi` so A9 `/hello` and later `/probe` readers stay stable; `/fwr` left for host-check.
- Single-cluster / FILE_MAX=256 first cut. spc=1 image.
