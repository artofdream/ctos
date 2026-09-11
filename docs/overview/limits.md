# Drawbacks / limits

What this project **is not**, and what is still unfinished. Pair with [KPIs / how we measure](measure.md) and [product vision](../01-vision/product-vision.md).

## Learning kernel, not a product

ctos is a research / teaching AArch64 kernel. It is not production-ready, not a desktop, not POSIX, not a container host, and not a “secure OS.” Do not treat a green QEMU smoke as certification.

## Boot contract still identity at `0x4008_0000`

QEMU `-kernel` and `_start` stay at `0x4008_0000`. High-VA work (TTBR1 alias, live `.text` tear after a vtable rewrite) does **not** mean the kernel moved. `.rodata` / `.data` / heap can still be identity-mapped. Full identity teardown is **Planned**. See [el0.md](../framework/el0.md) and [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md).

## PAN unclaimed on `cortex-a57`

Default probe CPU is `-cpu cortex-a57` (ARMv8.0). Do not claim Privileged Access Never. The [ledger](../framework/honesty-ledger.md) row stays **Planned** until `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1-vs-EL0 access fault is probed. Do not silently switch `-cpu`.

## No network, disk, or DMA

There is no NIC driver, no virtio-net, no block device, and no DMA API. Input on virt is PL011 UART RX. Timer is GICv2 + CNTP. That is the I/O surface.

## No real userspace apps

Standing EL0 is a mile, not an application runtime. No ELF loader for third-party programs, no libc, no SMP, no GPU. Concrete examples: [What can run today](what-can-run.md). How to extend the kernel (not port POSIX): [Building or porting](porting.md).

## Docs URL may lag the repo

The Route 53 CNAME for `ctos.artof.link` can be in place while GitHub Pages is still off. **Do not** claim `https://ctos.artof.link` serves these docs until Pages is enabled, a `main` deploy is green, and HTTPS fetches the book. Custom-domain reachability stays **Planned** until that probe. Details: [Docs website + DNS](../website.md).

## Other honest gaps

- Umbrella EL0 isolation: **Planned** (PAN + full identity teardown still missing).
- x86_64 is not a primary path.
- Raspberry Pi / other boards: unprobed.
- CI on GitHub is a separate ledger row from a cloud-VM `qemu-smoke`.
