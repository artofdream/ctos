//! Build the A2 hello payload (linked against `libctos`) and emit a
//! flat image the kernel copies onto `paging::EL0_PAGE`.
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
    let libctos_dir = manifest_dir.join("libctos");

    println!("cargo:rerun-if-changed={}", hello_dir.display());
    println!("cargo:rerun-if-changed={}", libctos_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    let elf = build_hello_rust(&manifest_dir, &hello_dir, &out_dir)
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
    assert_payload_svc(&image);

    let bin_path = out_dir.join("hello-libctos.bin");
    fs::write(&bin_path, &image).unwrap();
    fs::write(
        out_dir.join("hello_libctos_meta.rs"),
        format!(
            "pub const HELLO_LOAD_VA: u64 = {EL0_PAGE:#x};\n\
             pub const HELLO_LEN: usize = {};\n",
            image.len()
        ),
    )
    .unwrap();
    println!("cargo:rustc-env=CTOS_HELLO_LIBCTOS_LEN={}", image.len());
}

fn build_hello_rust(root: &Path, hello_dir: &Path, out_dir: &Path) -> Result<Vec<u8>, String> {
    let target_dir = out_dir.join("hello-target");
    let target_json = hello_dir.join("aarch64-ctos-user.json");
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let linker = hello_dir.join("linker.ld");
    let rustflags = format!("-C\x1flink-arg=-T{}", linker.display());

    let status = Command::new(&cargo)
        .args([
            "build",
            "--release",
            "--manifest-path",
        ])
        .arg(hello_dir.join("Cargo.toml"))
        .arg("--target")
        .arg(&target_json)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS")
        .env("CARGO_ENCODED_RUSTFLAGS", rustflags)
        .current_dir(root)
        .status()
        .map_err(|e| format!("spawn cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build hello-libctos exited {status}"));
    }

    let elf_path = target_dir
        .join("aarch64-ctos-user")
        .join("release")
        .join("hello-libctos");
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

fn assert_payload_svc(image: &[u8]) {
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
