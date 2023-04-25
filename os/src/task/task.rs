//! Types related to task management & Functions for completely changing TCB

use super::id::TaskUserRes;
use super::{kstack_alloc, KernelStack, ProcessControlBlock, TaskContext};
use crate::trap::TrapContext;
use crate::{mm::PhysPageNum, sync::UPSafeCell};
use super::stride::Stride;
use alloc::sync::{Arc, Weak};
use core::cell::RefMut;
use crate::config::{MAX_SYSCALL_NUM, INIT_PRIORITY, BIGSTRDE};

use alloc::boxed::Box;

#[allow(unused)]
pub struct SyscallInfo {
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time: usize,
    pub is_first :bool
}

#[allow(unused)]
impl SyscallInfo {
    pub fn zero_init() -> Self {
        Self {
            syscall_times: [0; MAX_SYSCALL_NUM],
            time: 0,
            is_first : true,
        }
    }
}

#[derive(Clone,Copy)]
pub struct StrideInfo{
    pub stride: Stride,
    pub priority:u64
}

impl StrideInfo {
    pub fn new() -> Self {
        Self {
            stride: Stride::init(),
            priority: INIT_PRIORITY,
        }
    }
    pub fn step(&mut self) {
        self.stride.0+=BIGSTRDE/self.priority;
    }
}

/// Task control block structure
pub struct TaskControlBlock {
    /// immutable
    pub process: Weak<ProcessControlBlock>,
    /// Kernel stack corresponding to PID
    pub kstack: KernelStack,
    /// mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    /// Get the mutable reference of the inner TCB
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// Get the address of app's page table
    pub fn get_user_token(&self) -> usize {
        let process = self.process.upgrade().unwrap();
        let inner = process.inner_exclusive_access();
        inner.memory_set.token()
    }
}

pub struct TaskControlBlockInner {
    pub res: Option<TaskUserRes>,
    /// The physical page number of the frame where the trap context is placed
    pub trap_cx_ppn: PhysPageNum,
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,
    /// It is set when active exit or execution error occurs
    pub exit_code: Option<i32>,

    /// use for lab1
    pub syscall_info:Box<SyscallInfo>,

    pub stride_info:StrideInfo
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    #[allow(unused)]
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
}

impl TaskControlBlock {
    /// Create a new task
    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = TaskUserRes::new(Arc::clone(&process), ustack_base, alloc_user_res);
        let trap_cx_ppn = res.trap_cx_ppn();
        let kstack = kstack_alloc();
        let kstack_top = kstack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kstack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    res: Some(res),
                    trap_cx_ppn,
                    task_cx: TaskContext::goto_trap_return(kstack_top),
                    task_status: TaskStatus::Ready,
                    exit_code: None,
                    syscall_info:Box::new(SyscallInfo::zero_init()),
                    stride_info: StrideInfo::new(),
                })
            },
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// The execution status of the current process
pub enum TaskStatus {
    /// ready to run
    Ready,
    /// running
    Running,
    /// blocked
    Blocked,
}
