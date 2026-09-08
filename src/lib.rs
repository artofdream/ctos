use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: usize,
    pub length: usize,
}

impl MemoryRegion {
    pub fn new(start: usize, length: usize) -> Self {
        Self { start, length }
    }

    pub fn end(&self) -> usize {
        self.start.saturating_add(self.length)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Sleeping,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub id: u64,
    pub priority: u8,
    pub state: ProcessState,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct KernelConfig {
    pub memory_size: usize,
    pub kernel_start: usize,
    pub kernel_end: usize,
}

impl KernelConfig {
    pub fn memory_regions(&self) -> Vec<MemoryRegion> {
        let mut regions = Vec::new();
        if self.kernel_start > 0 {
            regions.push(MemoryRegion::new(0, self.kernel_start));
        }
        if self.kernel_end > self.kernel_start {
            regions.push(MemoryRegion::new(self.kernel_start, self.kernel_end - self.kernel_start));
        }
        if self.kernel_end < self.memory_size {
            regions.push(MemoryRegion::new(self.kernel_end, self.memory_size - self.kernel_end));
        }
        regions
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Scheduler {
    ready_queue: VecDeque<Process>,
    tick: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&mut self, process: Process) {
        self.ready_queue.push_back(process);
    }

    pub fn next(&mut self) -> Option<Process> {
        let process = self.ready_queue.pop_front();
        if process.is_none() {
            self.tick += 1;
        }
        process
    }

    pub fn len(&self) -> usize {
        self.ready_queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ready_queue.is_empty()
    }
}

pub fn initialize_kernel(memory_size: usize, kernel_start: usize, kernel_end: usize) -> KernelConfig {
    let config = KernelConfig {
        memory_size,
        kernel_start,
        kernel_end,
    };
    let _ = config.memory_regions();
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_regions_cover_kernel_and_available_space() {
        let config = initialize_kernel(1024, 128, 512);
        let regions = config.memory_regions();

        assert_eq!(regions.len(), 3);
        assert_eq!(regions[0], MemoryRegion::new(0, 128));
        assert_eq!(regions[1], MemoryRegion::new(128, 384));
        assert_eq!(regions[2], MemoryRegion::new(512, 512));
    }

    #[test]
    fn scheduler_runs_ready_processes_in_fifo_order() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(Process {
            id: 1,
            priority: 10,
            state: ProcessState::Ready,
        });
        scheduler.schedule(Process {
            id: 2,
            priority: 5,
            state: ProcessState::Ready,
        });

        assert_eq!(scheduler.len(), 2);
        assert_eq!(scheduler.next(), Some(Process {
            id: 1,
            priority: 10,
            state: ProcessState::Ready,
        }));
        assert_eq!(scheduler.next(), Some(Process {
            id: 2,
            priority: 5,
            state: ProcessState::Ready,
        }));
        assert!(scheduler.is_empty());
    }
}
