fn main() {
    let config = ctos::initialize_kernel(4096, 512, 1024);
    println!(
        "kernel initialized: {} bytes total, {} bytes in use",
        config.memory_size,
        config.kernel_end.saturating_sub(config.kernel_start)
    );
}
