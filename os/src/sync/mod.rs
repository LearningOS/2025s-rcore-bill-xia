//! Synchronization and interior mutability primitives

mod condvar;
mod mutex;
mod semaphore;
mod up;

pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking, MutexSpin, have_deadlock_mutex};
pub use semaphore::{Semaphore, have_deadlock_semaphore};
pub use up::UPSafeCell;
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use crate::task::TaskControlBlock;

/// Trait for deadlock detection, used by both mutex and Semaphore.
pub trait SyncResource {
    /// get the avaiable matrix
    fn get_available(&self) -> isize;
    /// get one row of the allocation matrix for thread tid
    fn get_allocation(&self, tid: usize) -> isize;
    /// get one row of the allocation need for thread tid
    fn get_need(&self, tid: usize) -> isize;
    /// mark need
    fn mark_need(&self);
    /// unmark need
    fn unmark_need(&self);
}

/// detect deadlock
pub fn have_deadlock(collect_available: impl FnOnce() -> VecDeque<isize>, collect_alloc: impl Fn(&Option<Arc<TaskControlBlock>>) -> VecDeque<isize>, collect_need: impl Fn(&Option<Arc<TaskControlBlock>>) -> VecDeque<isize>, task_list: &Vec<Option<Arc<TaskControlBlock>>>) -> bool {
    let available: VecDeque<isize> = collect_available();
    let allocation: VecDeque<VecDeque<isize>> = task_list
        .iter().map(|item| {
            collect_alloc(item)
        }).collect();
    let need: VecDeque<VecDeque<isize>> = task_list
        .iter().map(|item| {
            collect_need(item)
        }).collect();
    let mut work: VecDeque<isize> = available.clone();
    let mut finished: VecDeque<bool> = task_list.iter().map(|_| {false}).collect();
    loop {
        let mut found_task = false;
        for i in 0..task_list.len() {
            if finished[i] {
                continue;
            }
            if work.iter().enumerate().any(|(j, _)| {
                need[i][j] > work[j]
            }) {
                continue;
            }
            found_task = true;
            finished[i] = true;
            for j in 0..work.len() {
                work[j] += allocation[i][j]
            }
            break;
        }
        if finished.iter().all(|x| {*x}) {
            return false
        }
        if !found_task {
            break;
        }
    }
    true
}