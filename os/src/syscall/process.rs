//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        exit_current_and_run_next, suspend_current_and_run_next, current_user_token, TaskStatus, current_task, add_task, task_mmap, task_munmap, manager::get_task_num,
    }, 
    timer::get_time_us, mm::{copy_bytes, VirtAddr, MapPermission},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    pub fn from_us(ut:usize) -> Self{
        return Self{ sec: ut / 1000000, usec: ut % 1000000 };
    }
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    
    let token = current_user_token();
    let path = translated_str(token, path);
    trace!("kernel:pid[{}] sys_exec , name is : {}", current_task().unwrap().pid.0,path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    trace!("current task num is {}",get_task_num());
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let token=current_user_token();
    let time_us =get_time_us();
    let ts = TimeVal::from_us(time_us);
    let res=copy_bytes(token,&ts,_ts as *mut u8);
    trace!("kernel: sys_get_time");
    trace!("kernel::pid[{}] sys_get_time {} us", current_task().unwrap().pid.0,time_us);
    res
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap called",
        current_task().unwrap().pid.0
    );

    if _start & PAGE_SIZE-1 != 0 {
        trace!("kernel: sys_mmap _start not align!");
        return -1;
    }
    
    if _port & !0x7 != 0 || _port & 0x7 == 0 {
        trace!("kernel: sys_mmap _port not fit!");
        return -1;
    }

    // do nothing
    if _len==0 {
        return 0;
    }

    let s_va= VirtAddr::from(_start);
    let e_va = VirtAddr::from(_start+_len);
    

    let mut flag = MapPermission::empty();
    if (_port & 0b001) != 0 {
        flag |= MapPermission::R;
    }
    if (_port & 0b010) != 0 {
        flag |= MapPermission::W;
    }
    if (_port & 0b100) != 0 {
        flag |= MapPermission::X;
    }

    task_mmap(s_va,e_va,flag)
    
    
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap called",
        current_task().unwrap().pid.0
    );
    // do nothing
    if _len==0 {
        return 0;
    }
    
    if _start & PAGE_SIZE-1 != 0 {
        trace!("kernel: sys_munmap _start not align!");
        return -1;
    }

    let s_va= VirtAddr::from(_start);
    let e_va = VirtAddr::from(_start+_len);


    task_munmap(s_va,e_va)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    
    let token = current_user_token();
    let path = translated_str(token, _path);
    trace!(
        "kernel:pid[{}] sys_spawn ,name is : {}",
        current_task().unwrap().pid.0,path
    );
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data);
        let new_pid = new_task.pid.0;
        // modify trap context of new_task, because it returns immediately after switching
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        // we do not have to move to next instruction since we have done it before
        // for child process, fork returns 0
        trap_cx.x[10] = 0;
        // add new task to scheduler
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }


}

// YOUR JOB: Set task priority.
///lab5
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority run",
        current_task().unwrap().pid.0
    );
    if _prio>=2 {
        let task=current_task().unwrap();
        let mut cur_task_inner=task.inner_exclusive_access();
        cur_task_inner.stride_info.priority=_prio.try_into().unwrap();
        return _prio;
    }
    error!("priority didn't fit");
    -1
}
