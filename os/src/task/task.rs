//! Types related to task management

use alloc::{vec::{Vec, self}, boxed::Box};

use super::TaskContext;
// use crate::syscall::process;
use crate::config::{ MAX_SYSCALL_NUM, MAX_APP_NUM};

#[derive(Copy, Clone,Debug)]
pub struct SyscallInfo {
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time: usize,
    pub is_first :bool
}
impl SyscallInfo {
    pub fn zero_init() -> Self {
        Self {
            syscall_times: [0; MAX_SYSCALL_NUM],
            time: 0,
            is_first : true,
        }
    }
}
#[derive(Debug,Clone)]
/// task control block structure
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub task_info: Box<SyscallInfo>
    // LAB1: Add whatever you need about the Task.
}

#[derive(Copy, Clone, PartialEq,Debug)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
