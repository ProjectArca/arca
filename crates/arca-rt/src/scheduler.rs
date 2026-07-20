//! Work-stealing task scheduler and runtime task queue for Arca.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Cancelled,
}

#[derive(Clone)]
pub struct CancellationToken {
    is_cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }
}

pub type TaskBox = Box<dyn FnOnce() + Send + 'static>;

pub struct TaskScheduler {
    num_threads: usize,
    queues: Vec<Arc<Mutex<VecDeque<TaskBox>>>>,
    next_task_id: AtomicU64,
}

impl TaskScheduler {
    pub fn new(num_threads: usize) -> Self {
        let threads = if num_threads == 0 { 1 } else { num_threads };
        let mut queues = Vec::with_capacity(threads);
        for _ in 0..threads {
            queues.push(Arc::new(Mutex::new(VecDeque::new())));
        }

        Self {
            num_threads: threads,
            queues,
            next_task_id: AtomicU64::new(1),
        }
    }

    pub fn spawn<F>(&self, thread_idx: usize, task_fn: F) -> TaskId
    where
        F: FnOnce() + Send + 'static,
    {
        let id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
        let target_idx = thread_idx % self.num_threads;

        if let Ok(mut q) = self.queues[target_idx].lock() {
            q.push_back(Box::new(task_fn));
        }

        TaskId(id)
    }

    pub fn execute_work_stealing(&self) -> usize {
        let mut completed_tasks = 0;
        let mut handles = Vec::new();

        for (thread_id, queue) in self.queues.iter().enumerate() {
            let q_clone = Arc::clone(queue);
            let peer_queues: Vec<Arc<Mutex<VecDeque<TaskBox>>>> = self
                .queues
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx != thread_id)
                .map(|(_, q)| Arc::clone(q))
                .collect();

            let handle = thread::spawn(move || {
                let mut local_completed = 0;

                loop {
                    // 1. Pop from local queue
                    let mut task_opt = None;
                    if let Ok(mut q) = q_clone.lock() {
                        task_opt = q.pop_front();
                    }

                    // 2. Steal from peer queue if local queue is empty
                    if task_opt.is_none() {
                        for peer_q in &peer_queues {
                            if let Ok(mut pq) = peer_q.lock() {
                                if let Some(stolen) = pq.pop_back() {
                                    task_opt = Some(stolen);
                                    break;
                                }
                            }
                        }
                    }

                    if let Some(task) = task_opt {
                        task();
                        local_completed += 1;
                    } else {
                        break; // All queues drained
                    }
                }

                local_completed
            });

            handles.push(handle);
        }

        for h in handles {
            if let Ok(count) = h.join() {
                completed_tasks += count;
            }
        }

        completed_tasks
    }
}
