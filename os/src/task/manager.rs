//!Implementation of [`TaskManager`]
use core::isize;

use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
/// An array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// the maximum stride for stride scheduler
pub const BIG_STRIDE: isize = 0x10000000;

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut i: usize = 0;
        let mut min_stride: isize = isize::MAX;
        let mut min_stride_idx: usize = 0;
        let mut prio: isize = 0;
        for task in &self.ready_queue {
            let inner = task.inner_exclusive_access();
            let curr_stride = inner.stride;
            if curr_stride < min_stride {
                min_stride = curr_stride;
                min_stride_idx = i;
                prio = inner.prio;
            }
            i += 1;
        }
        let task = self.ready_queue.swap_remove_back(min_stride_idx).unwrap();
        task.inner_exclusive_access().stride += (BIG_STRIDE / prio) + 1;
        Some(task)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
