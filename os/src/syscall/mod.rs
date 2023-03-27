//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_TASK_INFO: usize = 410;

mod fs;
mod process;

use fs::*;
use process::*;

use crate::{task::TASK_MANAGER, timer::{get_time, get_time_us}};

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    // LAB1: You may need to update syscall info here.
    let mut inner = TASK_MANAGER.get_inner();
    let c_task_id=inner.current_task;
    inner.tasks[c_task_id].task_info.syscall_times[syscall_id]+=1;
    drop(inner);
    let result=match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => {
            // let mut inner = TASK_MANAGER.get_inner();
            // let c_task_id=inner.current_task;
            // inner.tasks[c_task_id].task_info.time+=get_time()-inner.tasks[c_task_id].task_info.c_ready_start_time;

            let res=sys_yield();
            
            res
        },
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };
    
    result
}
