//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.

use super::{ProcessControlBlock, TaskControlBlock, TaskStatus};

use super::stride::Stride;
use crate::sync::UPSafeCell;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    
    /// The stopping task, leave a reference so that the kernel stack will not be recycled when switching tasks
    stop_task: Option<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            stop_task: None,
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if self.ready_queue.is_empty() {
            error!("fetch can't get, queue is empty");
            return None;
        }
        let mut next_task=self.ready_queue.pop_front().unwrap();
        for _ in 0..self.ready_queue.len() {
            let mid_task = self.ready_queue.pop_front().unwrap();
            if next_task.inner_exclusive_access().stride_info.stride > mid_task.inner_exclusive_access().stride_info.stride {
                self.ready_queue.push_back(next_task);
                next_task=mid_task;
            }
            else {
                self.ready_queue.push_back(mid_task);
            }
        }

        //debug
        trace!("\n\nfetch_task");
        trace!("choose pid[{}] with stride {}",next_task.process.upgrade().unwrap().getpid(),next_task.inner_exclusive_access().stride_info.stride.0);
        trace!("other task stride is:");
        for t in self.ready_queue.iter() {
            trace!("pid[{}] with stride {}",t.process.upgrade().unwrap().getpid(),t.inner_exclusive_access().stride_info.stride.0);
        }
        trace!("\n");

        let mut inner=next_task.inner_exclusive_access();
        let stride_info=&mut inner.stride_info;
        stride_info.step();
        // assert!(stride_info.stride>0);
        // drop(stride_info);
        drop(inner);

        // let stride_info=&mut next_task.inner_exclusive_access().stride_info;
        // stride_info.stride += BIGSTRDE/stride_info.priority;
        // assert!(stride_info.stride>0);
        // drop(stride_info);

        Some(next_task)
    }
    pub fn remove(&mut self, task: Arc<TaskControlBlock>) {
        if let Some((id, _)) = self
            .ready_queue
            .iter()
            .enumerate()
            .find(|(_, t)| Arc::as_ptr(t) == Arc::as_ptr(&task))
        {
            self.ready_queue.remove(id);
        }
    }
    /// Add a task to stopping task
    pub fn add_stop(&mut self, task: Arc<TaskControlBlock>) {
        // NOTE: as the last stopping task has completely stopped (not
        // using kernel stack any more, at least in the single-core
        // case) so that we can simply replace it;
        self.stop_task = Some(task);
    }

    pub fn get_min_stride(&self) -> Stride {
        if let Some(s)= self.ready_queue.iter().min_by_key(|x | x.inner_exclusive_access().stride_info.stride) {
            return s.inner_exclusive_access().stride_info.stride;
        } 
        return Stride(0);
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
    /// PID2PCB instance (map of pid to pcb)
    pub static ref PID2PCB: UPSafeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UPSafeCell::new(BTreeMap::new()) };
}

/// Add a task to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Wake up a task
pub fn wakeup_task(task: Arc<TaskControlBlock>) {
    trace!("kernel: TaskManager::wakeup_task");
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);
}

/// Remove a task from the ready queue
pub fn remove_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::remove_task");
    TASK_MANAGER.exclusive_access().remove(task);
}

/// Fetch a task out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// Set a task to stop-wait status, waiting for its kernel stack out of use.
pub fn add_stopping_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add_stop(task);
}

/// Get process by pid
pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.exclusive_access();
    map.get(&pid).map(Arc::clone)
}

/// Insert item(pid, pcb) into PID2PCB map (called by do_fork AND ProcessControlBlock::new)
pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.exclusive_access().insert(pid, process);
}

/// Remove item(pid, _some_pcb) from PDI2PCB map (called by exit_current_and_run_next)
pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.exclusive_access();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid);
    }
}
pub fn get_task_num() -> usize {
    TASK_MANAGER.exclusive_access().ready_queue.len()
}

pub fn get_min_stride() -> Stride {
    TASK_MANAGER.exclusive_access().get_min_stride()
}
