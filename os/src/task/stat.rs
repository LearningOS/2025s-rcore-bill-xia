//! Implementation of [`TaskStatistics`]

use crate::syscall::{SYSCALL_EXIT,SYSCALL_GET_TIME,SYSCALL_TRACE,SYSCALL_WRITE,SYSCALL_YIELD};

const SYSCALL_NUM: usize = 5;

#[derive(Copy, Clone)]
#[repr(C)]
/// task statistics structure containing
/// syscall counts
pub struct TaskStatistics {
    /// Syscall counting, indexed by syscall id
    syscall_count: [usize; SYSCALL_NUM],
}

impl TaskStatistics {
    /// A syscall with `syscall_id` happened
    pub fn count(&mut self, syscall_id: usize) {
        for i in 0..SYSCALL_NUM {
            trace!("count: syscall_count[{}]={}", i, self.syscall_count[i]);
        }
        match syscall_id {
            SYSCALL_EXIT => self.syscall_count[0] += 1,
            SYSCALL_GET_TIME => self.syscall_count[1] += 1,
            SYSCALL_TRACE => self.syscall_count[2] += 1,
            SYSCALL_WRITE => self.syscall_count[3] += 1,
            SYSCALL_YIELD => self.syscall_count[4] += 1,
            _ => panic!("Invalid syscall_id")
        }
    }

    /// Get count of syscall `syscall_id`
    pub fn get_count(&self, syscall_id: usize) -> usize {
        for i in 0..SYSCALL_NUM {
            trace!("get_count: syscall_count[{}]={}", i, self.syscall_count[i]);
        }
        match syscall_id {
            SYSCALL_EXIT => self.syscall_count[0],
            SYSCALL_GET_TIME => self.syscall_count[1],
            SYSCALL_TRACE => self.syscall_count[2],
            SYSCALL_WRITE => self.syscall_count[3],
            SYSCALL_YIELD => self.syscall_count[4],
            _ => 0
        }
    }

    /// Create a new empty task statistics structure
    pub fn zero_init() -> Self {
        Self {
            syscall_count: [0; SYSCALL_NUM],
        }
    }
}