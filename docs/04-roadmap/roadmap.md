# Roadmap

Each row is **one loop unit**: one milestone, one branch, one GitHub PR. Do not stack a later stage onto an unclosed earlier PR.

Order follows the Writing an OS in Rust series (VGA → tests → exceptions → interrupts → paging → heap → tasks).

| ID | Milestone | Probe that closes it | Status |
| --- | --- | --- | --- |
| M0 | Kernel tree + harness scaffold on GitHub | Files on the default-target PR; ledger started | In this PR (source). Merge is a human/MRC job. |
| M1 | QEMU boots VGA "Hello World!" | QEMU monitor dump of VGA row 23 (`0xb8e60`) reads the string; see honesty ledger | Verified in the 2026-09-08 cloud probe (not CI) |
| M2 | Integration test harness | QEMU isa-debug-exit + at least one `#[test_case]` that fails closed | Planned |
| M3 | IDT + breakpoint exception | Test or QEMU serial proof the handler runs | Planned |
| M4 | Double fault (IST) | Double-fault path does not triple-fault the VM | Planned |
| M5 | Hardware interrupts (PIC + timer) | Timer tick observable (serial or test) | Planned |
| M6 | Keyboard | Scancode path prints or tests a key | Planned |
| M7 | Paging + frame allocator | Map/unmap or allocator test | Planned |
| M8 | Heap (`alloc`) | Box/vec smoke on the heap | Planned |
| M9 | Cooperative scheduler | Two tasks observed to run | Planned |

M0 and M1 landed in the same scaffold PR because the cloud environment could run QEMU. That does not mean later milestones may stack. M2+ stay one PR each. Merge of this PR is still a human/MRC job.
