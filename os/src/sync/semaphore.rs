//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_process, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc, collections::BTreeMap, collections::btree_map::Entry};

use super::{have_deadlock, SyncResource};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub using: BTreeMap<usize, isize>,
    pub total: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                    using: BTreeMap::new(),
                    total: res_count as isize
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        } else {
            let curr_tid = current_task().unwrap().get_tid().unwrap();
            match inner.using.entry(curr_tid) {
                Entry::Occupied(mut entry) => {
                    *entry.get_mut() -= 1;
                    if *entry.get() == 0 {
                        entry.remove();
                    }
                }
                _ => {}
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        } else {
            let curr_tid = current_task().unwrap().get_tid().unwrap();
            *inner.using.entry(curr_tid).or_insert(0) += 1;
        }
    }
}

impl SyncResource for Semaphore {
    fn get_available(&self) -> isize {
        0.max(self.inner.exclusive_access().count)
    }
    fn get_allocation(&self, tid: usize) -> isize {
        *self.inner.exclusive_access().using.get(&tid).unwrap_or(&0)
    }
    fn get_need(&self, tid: usize) -> isize {
        self.inner.exclusive_access().wait_queue.iter().map(|task: &Arc<TaskControlBlock>| {
            if task.get_tid() == Some(tid) {
                1
            } else {
                0
            }
        }).sum::<isize>()
    }

    fn mark_need(&self) {
        self.inner.exclusive_access().wait_queue.push_back(current_task().unwrap());
    }

    fn unmark_need(&self) {
        self.inner.exclusive_access().wait_queue.pop_back();
    }
}

/// detect deadlock
pub fn have_deadlock_semaphore(sem: &Arc<Semaphore>) -> bool {
    let proc = current_process();
    let proc_inner = proc.inner_exclusive_access();
    sem.mark_need();

    let res = have_deadlock(|| {
        proc_inner.semaphore_list.iter().map(|item| {
            match item {
                Some(res) => res.get_available(),
                _ => 0
            }
        }).collect()
    }, |item| {
        match item {
            Some(task) => {
                let _tid = task.get_tid();
                if let Some(tid) = _tid {
                    proc_inner.semaphore_list.iter().map(|item| {
                        match item {
                            Some(res) => res.get_allocation(tid),
                            None => 0
                        }
                    }).collect()
                } else {
                    proc_inner.semaphore_list.iter().map(|_| {0}).collect()
                }
            },
            None => proc_inner.semaphore_list.iter().map(|_| {0}).collect()
        }
    }, |item| {
        match item {
            Some(task) => {
                let _tid = task.get_tid();
                if let Some(tid) = _tid {
                    proc_inner.semaphore_list.iter().map(|item| {
                        match item {
                            Some(res) => res.get_need(tid),
                            None => 0
                        }
                    }).collect()
                } else {
                    proc_inner.semaphore_list.iter().map(|_| {0}).collect()
                }
            },
            _ => proc_inner.semaphore_list.iter().map(|_| {0}).collect()
        }
    }, &proc_inner.tasks);

    sem.unmark_need();
    res
}