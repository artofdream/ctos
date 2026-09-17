//! Build freestanding user payloads linked against `libctos`.
//!
//! A2 hello: flat image the kernel may copy onto `paging::EL0_PAGE`, plus
//! published `target/hello-libctos.elf` for FAT `/hello`.
//! ADR-059: also publish `target/fs-libctos.elf` for FAT `/fsdemo`.
//! ADR-061: also publish `target/fat-libctos.elf` for FAT `/fatdemo`.
//! ADR-062: also publish `target/yield-libctos.elf` for FAT `/yldemo`.
//! ADR-068: also publish `target/net-libctos.elf` for FAT `/netdemo`.
//! ADR-071: also publish `target/udp-libctos.elf` for FAT `/udpdemo`.
//! ADR-075: also publish `target/mkdir-libctos.elf` for FAT `/mkdemo`.
//!
//! This is **not** a guest ELF loader (A3). The parse is host-side only.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Must match `paging::EL0_PAGE`.
const EL0_PAGE: u64 = 0x8000_2000;
/// Leave room for SP on the same 4 KiB page as today's standing payload.
const MAX_PAYLOAD: usize = 3584;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let hello_dir = manifest_dir.join("user/hello-libctos");
    let fs_dir = manifest_dir.join("user/fs-libctos");
    let fat_dir = manifest_dir.join("user/fat-libctos");
    let yield_dir = manifest_dir.join("user/yield-libctos");
    let net_dir = manifest_dir.join("user/net-libctos");
    let udp_dir = manifest_dir.join("user/udp-libctos");
    let mkdir_dir = manifest_dir.join("user/mkdir-libctos");
    let libctos_dir = manifest_dir.join("libctos");

    println!("cargo:rerun-if-changed={}", hello_dir.display());
    println!("cargo:rerun-if-changed={}", fs_dir.display());
    println!("cargo:rerun-if-changed={}", fat_dir.display());
    println!("cargo:rerun-if-changed={}", yield_dir.display());
    println!("cargo:rerun-if-changed={}", net_dir.display());
    println!("cargo:rerun-if-changed={}", udp_dir.display());
    println!("cargo:rerun-if-changed={}", mkdir_dir.display());
    println!("cargo:rerun-if-changed={}", libctos_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    let elf = build_user_rust(&manifest_dir, &hello_dir, &out_dir, "hello-libctos")
        .or_else(|e| {
            println!("cargo:warning=hello-libctos rustc payload failed ({e}); trying asm fallback");
            build_hello_asm(&manifest_dir, &out_dir)
        })
        .unwrap_or_else(|e| panic!("libctos hello payload: {e}"));

    let (load_va, image) = elf64_pt_load(&elf).unwrap_or_else(|e| panic!("payload ELF: {e}"));
    if load_va != EL0_PAGE {
        panic!("payload load VA {load_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if image.is_empty() || image.len() > MAX_PAYLOAD {
        panic!(
            "payload size {} is empty or > {MAX_PAYLOAD} (must fit one EL0 page + stack)",
            image.len()
        );
    }
    assert_hello_payload(&image);

    let bin_path = out_dir.join("hello-libctos.bin");
    fs::write(&bin_path, &image).unwrap();
    // A3 guest loader consumes the ELF bytes (not the host-flattened image).
    let elf_path = out_dir.join("hello-libctos.elf");
    if elf.len() > 64 * 1024 {
        panic!("hello ELF {} bytes is > 64 KiB (keep the A3 embed small)", elf.len());
    }
    fs::write(&elf_path, &elf).unwrap();
    // A9 / ADR-030 + leftover mile (ADR-032): publish a host app
    // artifact next to the OS image. A2–A4 and production A9 read
    // FAT `/hello` (no include_bytes!). ADR-046 may include this
    // OUT_DIR copy from src/slot.rs for a probe-only CNTPCT pair.
    // A kernel rebuild still compiles the payload here.
    let published = manifest_dir.join("target/hello-libctos.elf");
    if let Some(parent) = published.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&published, &elf).unwrap();
    fs::write(
        out_dir.join("hello_libctos_meta.rs"),
        format!(
            "pub const HELLO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const HELLO_LEN: usize = {};\n\
             pub const HELLO_ELF_LEN: usize = {};\n",
            image.len(),
            elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_HELLO_LIBCTOS_LEN={}", image.len());
    println!("cargo:rustc-env=CTOS_HELLO_LIBCTOS_ELF_LEN={}", elf.len());

    // ADR-059: second freestanding sample (VFS wrappers). FAT `/fsdemo` only.
    let fs_elf = build_user_rust(&manifest_dir, &fs_dir, &out_dir, "fs-libctos")
        .unwrap_or_else(|e| panic!("libctos fs-libctos payload: {e}"));
    let (fs_va, fs_image) =
        elf64_pt_load(&fs_elf).unwrap_or_else(|e| panic!("fs-libctos ELF: {e}"));
    if fs_va != EL0_PAGE {
        panic!("fs-libctos load VA {fs_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if fs_elf.is_empty() || fs_elf.len() > 64 * 1024 {
        panic!("fs-libctos ELF {} bytes empty or > 64 KiB", fs_elf.len());
    }
    assert_fs_payload(&fs_image);
    fs::write(out_dir.join("fs-libctos.elf"), &fs_elf).unwrap();
    fs::write(manifest_dir.join("target/fs-libctos.elf"), &fs_elf).unwrap();
    fs::write(
        out_dir.join("fs_libctos_meta.rs"),
        format!(
            "pub const FSDEMO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const FSDEMO_ELF_LEN: usize = {};\n",
            fs_elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_FS_LIBCTOS_ELF_LEN={}", fs_elf.len());

    // ADR-061: third freestanding sample (FAT via thin VFS). FAT `/fatdemo` only.
    let fat_elf = build_user_rust(&manifest_dir, &fat_dir, &out_dir, "fat-libctos")
        .unwrap_or_else(|e| panic!("libctos fat-libctos payload: {e}"));
    let (fat_va, fat_image) =
        elf64_pt_load(&fat_elf).unwrap_or_else(|e| panic!("fat-libctos ELF: {e}"));
    if fat_va != EL0_PAGE {
        panic!("fat-libctos load VA {fat_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if fat_elf.is_empty() || fat_elf.len() > 64 * 1024 {
        panic!("fat-libctos ELF {} bytes empty or > 64 KiB", fat_elf.len());
    }
    assert_fat_payload(&fat_image);
    fs::write(out_dir.join("fat-libctos.elf"), &fat_elf).unwrap();
    fs::write(manifest_dir.join("target/fat-libctos.elf"), &fat_elf).unwrap();
    fs::write(
        out_dir.join("fat_libctos_meta.rs"),
        format!(
            "pub const FATDEMO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const FATDEMO_ELF_LEN: usize = {};\n",
            fat_elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_FAT_LIBCTOS_ELF_LEN={}", fat_elf.len());

    // ADR-062: fourth freestanding sample (cooperative yield rounds). FAT `/yldemo` only.
    let yield_elf = build_user_rust(&manifest_dir, &yield_dir, &out_dir, "yield-libctos")
        .unwrap_or_else(|e| panic!("libctos yield-libctos payload: {e}"));
    let (yield_va, yield_image) =
        elf64_pt_load(&yield_elf).unwrap_or_else(|e| panic!("yield-libctos ELF: {e}"));
    if yield_va != EL0_PAGE {
        panic!("yield-libctos load VA {yield_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if yield_elf.is_empty() || yield_elf.len() > 64 * 1024 {
        panic!("yield-libctos ELF {} bytes empty or > 64 KiB", yield_elf.len());
    }
    assert_yield_payload(&yield_image);
    fs::write(out_dir.join("yield-libctos.elf"), &yield_elf).unwrap();
    fs::write(manifest_dir.join("target/yield-libctos.elf"), &yield_elf).unwrap();
    fs::write(
        out_dir.join("yield_libctos_meta.rs"),
        format!(
            "pub const YLDEMO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const YLDEMO_ELF_LEN: usize = {};\n",
            yield_elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_YIELD_LIBCTOS_ELF_LEN={}", yield_elf.len());

    // ADR-068: fifth freestanding sample (EL0 net SVCs). FAT `/netdemo` only.
    let net_elf = build_user_rust(&manifest_dir, &net_dir, &out_dir, "net-libctos")
        .unwrap_or_else(|e| panic!("libctos net-libctos payload: {e}"));
    let (net_va, net_image) =
        elf64_pt_load(&net_elf).unwrap_or_else(|e| panic!("net-libctos ELF: {e}"));
    if net_va != EL0_PAGE {
        panic!("net-libctos load VA {net_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if net_elf.is_empty() || net_elf.len() > 64 * 1024 {
        panic!("net-libctos ELF {} bytes empty or > 64 KiB", net_elf.len());
    }
    assert_net_payload(&net_image);
    fs::write(out_dir.join("net-libctos.elf"), &net_elf).unwrap();
    fs::write(manifest_dir.join("target/net-libctos.elf"), &net_elf).unwrap();
    fs::write(
        out_dir.join("net_libctos_meta.rs"),
        format!(
            "pub const NETDEMO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const NETDEMO_ELF_LEN: usize = {};\n",
            net_elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_NET_LIBCTOS_ELF_LEN={}", net_elf.len());

    // ADR-071: sixth freestanding sample (EL0 UDP DNS SVC). FAT `/udpdemo` only.
    let udp_elf = build_user_rust(&manifest_dir, &udp_dir, &out_dir, "udp-libctos")
        .unwrap_or_else(|e| panic!("libctos udp-libctos payload: {e}"));
    let (udp_va, udp_image) =
        elf64_pt_load(&udp_elf).unwrap_or_else(|e| panic!("udp-libctos ELF: {e}"));
    if udp_va != EL0_PAGE {
        panic!("udp-libctos load VA {udp_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if udp_elf.is_empty() || udp_elf.len() > 64 * 1024 {
        panic!("udp-libctos ELF {} bytes empty or > 64 KiB", udp_elf.len());
    }
    assert_udp_payload(&udp_image);
    fs::write(out_dir.join("udp-libctos.elf"), &udp_elf).unwrap();
    fs::write(manifest_dir.join("target/udp-libctos.elf"), &udp_elf).unwrap();
    fs::write(
        out_dir.join("udp_libctos_meta.rs"),
        format!(
            "pub const UDPDEMO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const UDPDEMO_ELF_LEN: usize = {};\n",
            udp_elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_UDP_LIBCTOS_ELF_LEN={}", udp_elf.len());

    // ADR-075: seventh freestanding sample (EL0 fs_mkdir). FAT `/mkdemo` only.
    let mkdir_elf = build_user_rust(&manifest_dir, &mkdir_dir, &out_dir, "mkdir-libctos")
        .unwrap_or_else(|e| panic!("libctos mkdir-libctos payload: {e}"));
    let (mkdir_va, mkdir_image) =
        elf64_pt_load(&mkdir_elf).unwrap_or_else(|e| panic!("mkdir-libctos ELF: {e}"));
    if mkdir_va != EL0_PAGE {
        panic!("mkdir-libctos load VA {mkdir_va:#x} != EL0_PAGE {EL0_PAGE:#x}");
    }
    if mkdir_elf.is_empty() || mkdir_elf.len() > 64 * 1024 {
        panic!("mkdir-libctos ELF {} bytes empty or > 64 KiB", mkdir_elf.len());
    }
    assert_mkdir_payload(&mkdir_image);
    fs::write(out_dir.join("mkdir-libctos.elf"), &mkdir_elf).unwrap();
    fs::write(manifest_dir.join("target/mkdir-libctos.elf"), &mkdir_elf).unwrap();
    fs::write(
        out_dir.join("mkdir_libctos_meta.rs"),
        format!(
            "pub const MKDEMO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const MKDEMO_ELF_LEN: usize = {};\n",
            mkdir_elf.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_MKDIR_LIBCTOS_ELF_LEN={}", mkdir_elf.len());
}

fn build_user_rust(
    root: &Path,
    crate_dir: &Path,
    out_dir: &Path,
    bin_name: &str,
) -> Result<Vec<u8>, String> {
    let target_dir = out_dir.join(format!("{bin_name}-target"));
    let target_json = crate_dir.join("aarch64-ctos-user.json");
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let linker = crate_dir.join("linker.ld");
    let rustflags = format!("-C\x1flink-arg=-T{}", linker.display());

    let status = Command::new(&cargo)
        .args([
            "build",
            "--release",
            "--manifest-path",
        ])
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--target")
        .arg(&target_json)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS")
        .env("CARGO_ENCODED_RUSTFLAGS", rustflags)
        .current_dir(root)
        .status()
        .map_err(|e| format!("spawn cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build {bin_name} exited {status}"));
    }

    let elf_path = target_dir
        .join("aarch64-ctos-user")
        .join("release")
        .join(bin_name);
    fs::read(&elf_path).map_err(|e| format!("read {}: {e}", elf_path.display()))
}

fn build_hello_asm(root: &Path, out_dir: &Path) -> Result<Vec<u8>, String> {
    let mc = llvm_tool("llvm-mc")?;
    let ld = llvm_tool("rust-lld").or_else(|_| llvm_tool("ld.lld"))?;
    let asm_dir = out_dir.join("hello-asm");
    fs::create_dir_all(&asm_dir).map_err(|e| e.to_string())?;

    let stubs = assemble(&mc, &root.join("libctos/src/sys.S"), &asm_dir.join("sys.o"))?;
    let crt = assemble(&mc, &root.join("libctos/src/crt0.S"), &asm_dir.join("crt0.o"))?;
    let hello = assemble(
        &mc,
        &root.join("user/hello-libctos/src/hello.S"),
        &asm_dir.join("hello.o"),
    )?;
    let elf_path = asm_dir.join("hello-libctos");
    let status = Command::new(&ld)
        .args(["-flavor", "gnu", "-T"])
        .arg(root.join("user/hello-libctos/linker.ld"))
        .arg("-o")
        .arg(&elf_path)
        .args([&stubs, &crt, &hello])
        .status()
        .map_err(|e| format!("spawn lld: {e}"))?;
    if !status.success() {
        return Err(format!("lld hello-libctos exited {status}"));
    }
    fs::read(&elf_path).map_err(|e| format!("read {}: {e}", elf_path.display()))
}

fn assemble(mc: &Path, src: &Path, dst: &Path) -> Result<PathBuf, String> {
    let status = Command::new(mc)
        .args(["--arch=aarch64", "--filetype=obj", "-o"])
        .arg(dst)
        .arg(src)
        .status()
        .map_err(|e| format!("spawn llvm-mc: {e}"))?;
    if !status.success() {
        return Err(format!("llvm-mc {} failed", src.display()));
    }
    Ok(dst.to_path_buf())
}

fn llvm_tool(name: &str) -> Result<PathBuf, String> {
    if let Ok(p) = which(name) {
        return Ok(p);
    }
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let sysroot = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .map_err(|e| format!("rustc --print sysroot: {e}"))?;
    if !sysroot.status.success() {
        return Err("rustc --print sysroot failed".into());
    }
    let sysroot = String::from_utf8_lossy(&sysroot.stdout).trim().to_string();
    let bin = Path::new(&sysroot).join("lib/rustlib");
    if let Ok(entries) = fs::read_dir(&bin) {
        for host in entries.flatten() {
            let cand = host.path().join("bin").join(name);
            if cand.is_file() {
                return Ok(cand);
            }
        }
    }
    Err(format!("no {name} on PATH or in rustc sysroot"))
}

fn which(name: &str) -> Result<PathBuf, ()> {
    let path = env::var_os("PATH").ok_or(())?;
    for dir in env::split_paths(&path) {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(())
}

fn elf64_pt_load(elf: &[u8]) -> Result<(u64, Vec<u8>), String> {
    if elf.len() < 64 || &elf[0..4] != b"\x7fELF" {
        return Err("not ELF".into());
    }
    if elf[4] != 2 || elf[5] != 1 {
        return Err("need ELF64LE".into());
    }
    let phoff = u64::from_le_bytes(elf[32..40].try_into().unwrap()) as usize;
    let phentsize = u16::from_le_bytes(elf[54..56].try_into().unwrap()) as usize;
    let phnum = u16::from_le_bytes(elf[56..58].try_into().unwrap()) as usize;
    if phentsize < 56 || phoff == 0 {
        return Err("bad phdrs".into());
    }

    let mut loads = Vec::new();
    for i in 0..phnum {
        let off = phoff + i * phentsize;
        if off + 56 > elf.len() {
            return Err("phdr overflow".into());
        }
        let p_type = u32::from_le_bytes(elf[off..off + 4].try_into().unwrap());
        if p_type != 1 {
            continue;
        }
        let p_offset = u64::from_le_bytes(elf[off + 8..off + 16].try_into().unwrap());
        let p_vaddr = u64::from_le_bytes(elf[off + 16..off + 24].try_into().unwrap());
        let p_filesz = u64::from_le_bytes(elf[off + 32..off + 40].try_into().unwrap());
        let p_memsz = u64::from_le_bytes(elf[off + 40..off + 48].try_into().unwrap());
        loads.push((p_offset, p_vaddr, p_filesz, p_memsz));
    }
    // rust-lld also emits a PT_LOAD of ELF headers at 0x80000000.
    // The payload itself is linked at EL0_PAGE (0x80002000).
    loads.retain(|(_, vaddr, _, _)| *vaddr >= EL0_PAGE);
    if loads.is_empty() {
        return Err("no PT_LOAD at/after EL0_PAGE".into());
    }
    let min_va = loads.iter().map(|l| l.1).min().unwrap();
    let max_end = loads.iter().map(|l| l.1 + l.3).max().unwrap();
    let len = (max_end - min_va) as usize;
    let mut image = vec![0u8; len];
    for (p_offset, p_vaddr, p_filesz, _p_memsz) in loads {
        let dest = (p_vaddr - min_va) as usize;
        let src = p_offset as usize;
        let n = p_filesz as usize;
        if src + n > elf.len() || dest + n > image.len() {
            return Err("PT_LOAD copy out of range".into());
        }
        image[dest..dest + n].copy_from_slice(&elf[src..src + n]);
    }
    Ok((min_va, image))
}

fn assert_hello_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("payload encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    for need in [16u32, 17, 18] {
        if !words.contains(&svc(need)) {
            panic!("payload missing SVC #{need}");
        }
    }
    if !image.windows(12).any(|w| w == b"libctos: hi\n") {
        panic!("payload missing libctos: hi");
    }
    if !image.windows(12).any(|w| w == b"libctos: ok\n") {
        panic!("payload missing libctos: ok");
    }
}

fn assert_fs_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("fs-libctos encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    for need in [16u32, 17, 18, 19, 20, 21, 22, 23] {
        if !words.contains(&svc(need)) {
            panic!("fs-libctos missing SVC #{need}");
        }
    }
    if !image.windows(15).any(|w| w == b"libctos: fs-hi\n") {
        panic!("fs-libctos missing libctos: fs-hi");
    }
    if !image.windows(15).any(|w| w == b"libctos: fs-ok\n") {
        panic!("fs-libctos missing libctos: fs-ok");
    }
}

fn assert_fat_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("fat-libctos encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    // uart/yield/exit + open/read/close (create/write unused this mile).
    for need in [16u32, 17, 18, 20, 21, 23] {
        if !words.contains(&svc(need)) {
            panic!("fat-libctos missing SVC #{need}");
        }
    }
    if !image.windows(16).any(|w| w == b"libctos: fat-hi\n") {
        panic!("fat-libctos missing libctos: fat-hi");
    }
    if !image.windows(16).any(|w| w == b"libctos: fat-ok\n") {
        panic!("fat-libctos missing libctos: fat-ok");
    }
}

fn assert_yield_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("yield-libctos encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    for need in [16u32, 17, 18] {
        if !words.contains(&svc(need)) {
            panic!("yield-libctos missing SVC #{need}");
        }
    }
    if !image.windows(16).any(|w| w == b"libctos: yld-hi\n") {
        panic!("yield-libctos missing libctos: yld-hi");
    }
    if !image.windows(14).any(|w| w == b"libctos: beat\n") {
        panic!("yield-libctos missing libctos: beat");
    }
    if !image.windows(16).any(|w| w == b"libctos: yld-ok\n") {
        panic!("yield-libctos missing libctos: yld-ok");
    }
}

fn assert_net_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("net-libctos encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    // exit/uart/yield + net_mac/net_ping
    for need in [16u32, 17, 18, 24, 25] {
        if !words.contains(&svc(need)) {
            panic!("net-libctos missing SVC #{need}");
        }
    }
    if !image.windows(16).any(|w| w == b"libctos: net-hi\n") {
        panic!("net-libctos missing libctos: net-hi");
    }
    if !image.windows(17).any(|w| w == b"libctos: net-mac\n") {
        panic!("net-libctos missing libctos: net-mac");
    }
    if !image.windows(16).any(|w| w == b"libctos: net-ok\n") {
        panic!("net-libctos missing libctos: net-ok");
    }
}

fn assert_udp_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("udp-libctos encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    // exit/uart/yield + net_udp_dns
    for need in [16u32, 17, 18, 26] {
        if !words.contains(&svc(need)) {
            panic!("udp-libctos missing SVC #{need}");
        }
    }
    if !image.windows(16).any(|w| w == b"libctos: udp-hi\n") {
        panic!("udp-libctos missing libctos: udp-hi");
    }
    if !image.windows(16).any(|w| w == b"libctos: udp-ok\n") {
        panic!("udp-libctos missing libctos: udp-ok");
    }
}

fn assert_mkdir_payload(image: &[u8]) {
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: Vec<u32> = image
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    for reserved in [0u32, 1, 2] {
        if words.contains(&svc(reserved)) {
            panic!("mkdir-libctos encodes reserved SVC #{reserved} (ADR-013)");
        }
    }
    // exit/uart/yield + fs_mkdir
    for need in [16u32, 17, 18, 27] {
        if !words.contains(&svc(need)) {
            panic!("mkdir-libctos missing SVC #{need}");
        }
    }
    if !image.windows(18).any(|w| w == b"libctos: mkdir-hi\n") {
        panic!("mkdir-libctos missing libctos: mkdir-hi");
    }
    if !image.windows(18).any(|w| w == b"libctos: mkdir-ok\n") {
        panic!("mkdir-libctos missing libctos: mkdir-ok");
    }
}
