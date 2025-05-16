//! Mutex (spin-like and blocking(sleep))

use super::{have_deadlock, SyncResource, UPSafeCell};
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, current_process, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send + SyncResource {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    inner: UPSafeCell<MutexSpinInner>
}

pub struct MutexSpinInner {
    curr_task: Option<usize>,
    wait_queue: VecDeque<usize>, // [tid]
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
            inner: unsafe {
                UPSafeCell::new(MutexSpinInner {
                    curr_task: None,
                    wait_queue: VecDeque::new()
                })
            }
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        let curr_tid = current_task().unwrap().get_tid().unwrap();
        self.inner.exclusive_access().wait_queue.push_back(curr_tid);
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                // register for deadlock detection
                self.inner.exclusive_access().curr_task = Some(curr_tid);
                self.inner.exclusive_access().wait_queue.retain(|tid| {
                    *tid != curr_tid
                });
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;

        // unregister for deadlock detection
        let curr_tid = current_task().unwrap().get_tid().unwrap();
        self.inner.exclusive_access().curr_task = None;
        self.inner.exclusive_access().wait_queue.retain(|tid| {
            *tid != curr_tid
        });
    }
}

impl SyncResource for MutexSpin {
    fn get_available(&self) -> isize {
        (!*self.locked.exclusive_access()) as isize
    }
    fn get_allocation(&self, tid: usize) -> isize {
        (Some(tid) == self.inner.exclusive_access().curr_task) as isize
    }
    fn get_need(&self, tid: usize) -> isize {
        current_task().unwrap().get_tid().unwrap() as isize +
            self.inner.exclusive_access().wait_queue.iter().map(|t| {
                if *t == tid {1} else {0}
            }).sum::<isize>()
    }
    fn mark_need(&self) {
        let curr_tid = current_task().unwrap().get_tid().unwrap();
        self.inner.exclusive_access().wait_queue.push_back(curr_tid);
    }
    fn unmark_need(&self) {
        self.inner.exclusive_access().wait_queue.pop_back();
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    curr_task: Option<usize>,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    curr_task: None,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            let curr_tid = current_task().unwrap().get_tid().unwrap();
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            mutex_inner.curr_task = Some(curr_tid);
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
        mutex_inner.curr_task = None;
    }
}

impl SyncResource for MutexBlocking {
    fn get_available(&self) -> isize {
        (!self.inner.exclusive_access().locked) as isize
    }
    fn get_allocation(&self, tid: usize) -> isize {
        (Some(tid) == self.inner.exclusive_access().curr_task) as isize
    }
    fn get_need(&self, tid: usize) -> isize {
        self.inner.exclusive_access().wait_queue.iter().map(|task| {
            (task.get_tid() == Some(tid)) as isize
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
pub fn have_deadlock_mutex(mutex: &Arc<dyn Mutex>) -> bool {
    let proc = current_process();
    let proc_inner = proc.inner_exclusive_access();
    mutex.mark_need();

    let res = have_deadlock(|| {
        proc_inner.mutex_list.iter().map(|item| {
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
                    proc_inner.mutex_list.iter().map(|item| {
                        match item {
                            Some(res) => res.get_allocation(tid),
                            None => 0
                        }
                    }).collect()
                } else {
                    proc_inner.mutex_list.iter().map(|_| {0}).collect()
                }
            },
            None => proc_inner.mutex_list.iter().map(|_| {0}).collect()
        }
    }, |item| {
        match item {
            Some(task) => {
                let _tid = task.get_tid();
                if let Some(tid) = _tid {
                    proc_inner.mutex_list.iter().map(|item| {
                        match item {
                            Some(res) => res.get_need(tid),
                            None => 0
                        }
                    }).collect()
                } else {
                    proc_inner.mutex_list.iter().map(|_| {0}).collect()
                }
            },
            _ => proc_inner.mutex_list.iter().map(|_| {0}).collect()
        }
    }, &proc_inner.tasks);

    mutex.unmark_need();
    res
}
