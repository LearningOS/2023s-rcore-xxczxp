//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use super::stride::Stride;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

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
        trace!("choose pid[{}] with stride {}",next_task.getpid(),next_task.inner_exclusive_access().stride_info.stride.0);
        trace!("other task stride is:");
        for t in self.ready_queue.iter() {
            trace!("pid[{}] with stride {}",t.getpid(),t.inner_exclusive_access().stride_info.stride.0);
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

pub fn get_task_num() -> usize {
    TASK_MANAGER.exclusive_access().ready_queue.len()
}

pub fn get_min_stride() -> Stride {
    TASK_MANAGER.exclusive_access().get_min_stride()
}
