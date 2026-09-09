# Product vision

## What ctos is

ctos is a **learning and research** bare-metal OS kernel written in Rust for x86_64. The near-term product is a bootable image that owns the machine after the bootloader: VGA text, then interrupts, paging, a heap, and a tiny scheduler — in that order.

It exists so we can study kernel mechanics with a document-first harness: claims stay honest, failures become sensors, and agents do not merge their own work.

## What ctos is not

- Not a production operating system, desktop, or app runtime.
- Not a Linux distro, not POSIX, not a container host.
- Not a florist / commerce platform and not a clone of any shop case study.
- Not a claim that QEMU boot, CI, or hardware bring-up is finished until a probe says so.

## Success (current horizon)

A new session can read the vision, architecture, honesty ledger, and latest daily brief, then take **one** roadmap milestone to a GitHub PR without inventing status.
