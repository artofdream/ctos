# Rust OS landscape — lessons for ctos (2026-09-09)

Side research while ctos retargets to **arm64 primary**. Goal: learn from others — what to look **for** and **against**. Not a leaderboard; not a claim that projects “failed.”

## Still going (worth watching)

| Project | Goal | What worked | Watch / don’t copy blindly |
| --- | --- | --- | --- |
| [Redox](https://www.redox-os.org/) | Unix-like microkernel, desktop/cloud alternative | Decade of grind; `std`/crate ecosystem; real ports | “Replace Linux” = lifetime + convince every language ecosystem. Porting/fork tax is huge. |
| [Asterinas](https://github.com/asterinas/asterinas) | Framekernel + Linux ABI compatibility | Clear niche (compat + confidential computing); ongoing releases | Compat OS ≠ small teaching kernel. |
| [Theseus](https://github.com/theseus-os/Theseus) | Intralingual design — push OS duties into the compiler | Strong papers; live evolution / fault recovery ideas | Research OS; all-Rust apps; ARM historically WIP. Steal ideas, not product scope. |
| [Tock](https://www.tockos.org/) | Embedded, dynamic apps, MPU isolation | Research → real devices at scale | Embedded/userspace-C story ≠ general-purpose. |
| [Hubris](https://github.com/oxidecomputer/hubris) (Oxide) | Static tasks, robust embedded | Narrow use case; static allocation; restart without resource chaos | No dynamic load by design. Wrong if the goal is a hobby desktop OS. |
| phil-opp / blog_os | Teach OS in Rust | Stepwise curriculum; tests + QEMU exit | x86-centric. ctos is **arm64** — steal the *loop* (milestone → probe), not VGA/`bootloader` 0.9 forever. |

Also in the zoo: Hermit (unikernel), Aero and many GitHub “minimal kernels” (often stall at console/heap).

## Why projects stall (patterns)

1. **Scope inflation** — hello → “Linux alternative” without a decade and a porting org.
2. **Ecosystem chicken/egg** — apps need `std`/POSIX; `std` needs a real OS personality.
3. **Allocator–scheduler–lock tango** — circular deps; fragile stubs; `unsafe` at the bottom forever.
4. **No sensors** — “boots on my machine” with no QEMU exit / CI → regressions win.
5. **ISA / board thrash** — rewriting boot every month instead of freezing one probe (`virt` + UART for ctos).
6. **Research vs product mix** — novelty without a ship constraint, or ship dates on research scope.

## Look for (steal these)

- One **clear mission** (learn / research / embedded / Linux-ABI) written down — FR/NFR + ADRs.
- **Fail-closed probes** (integration tests, Docker, GHA).
- **Document-first + honesty** — Verified only with a probe.
- Small **unsafe surface**; interrupt paths that don’t unwind.
- **One milestone → one PR**.

## Look against (smell tests)

- Roadmaps that jump to GUI/net/userspace before exceptions + memory + tests.
- Dual-ISA “for compatibility” before one ISA is boring.
- Claiming memory safety = secure OS (no threat model).
- Copying Redox/Asterinas goals into a teaching kernel.
- Dynamic everything early without choosing Hubris-vs-Tock on purpose.
- Unprobed “Verified” in docs.

## For ctos

Stay a **learning/research aarch64 kernel** with a hard harness. Borrow phil-opp’s pedagogy, Hubris’s “narrow + tested,” Redox’s honesty about how long a “real OS” takes — and avoid Redox/Asterinas product scope until FR/NFR say so.

## Sources (starting points)

- https://www.redox-os.org/
- https://github.com/theseus-os/Theseus
- https://www.tockos.org/
- https://github.com/oxidecomputer/hubris
- https://github.com/asterinas/asterinas
- https://github.com/programmerjake/rust-os-comparison
- https://os.phil-opp.com/
- https://blog.wellosoft.net/writing-a-brand-new-os-is-almost-impossible-by-now
