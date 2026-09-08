# Roadmap

Each row is **one loop unit**: one milestone, one branch, one GitHub PR. Do not stack a later stage onto an unclosed earlier PR.

Order follows the Writing an OS in Rust series (VGA → tests → exceptions → interrupts → paging → heap → tasks).

| ID | Milestone | Probe that closes it | Status |
| --- | --- | --- | --- |
| M0 | Kernel tree + harness scaffold on GitHub | Files on the default-target PR; ledger started | In this PR (source). Merge is a human/MRC job. |
| M1 | QEMU boots VGA "Hello World!" | `cargo run` (or `bootimage` + QEMU) shows the line; note the probe in the ledger | Unknown until probed |
| M2 | Integration test harness | QEMU isa-debug-exit + at least one `#[test_case]` that fails closed | Planned |
| M3 | IDT + breakpoint exception | Test or QEMU serial proof the handler runs | Planned |
| M4 | Double fault (IST) | Double-fault path does not triple-fault the VM | Planned |
| M5 | Hardware interrupts (PIC + timer) | Timer tick observable (serial or test) | Planned |
| M6 | Keyboard | Scancode path prints or tests a key | Planned |
| M7 | Paging + frame allocator | Map/unmap or allocator test | Planned |
| M8 | Heap (`alloc`) | Box/vec smoke on the heap | Planned |
| M9 | Cooperative scheduler | Two tasks observed to run | Planned |

M0 does not close M1. Shipping source is not a boot probe.
