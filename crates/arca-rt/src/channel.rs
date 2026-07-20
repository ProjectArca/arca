//! Bounded/Unbounded Channel and Actor Mailbox implementation for Arca.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Channel<T> {
    inner: Arc<Mutex<VecDeque<T>>>,
    capacity: Option<usize>,
}

impl<T> Channel<T> {
    pub fn new(capacity: Option<usize>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
            capacity,
        }
    }

    pub fn send(&self, val: T) -> Result<(), String> {
        if let Ok(mut q) = self.inner.lock() {
            if let Some(cap) = self.capacity {
                if q.len() >= cap {
                    return Err(format!("Channel capacity limit reached ({})", cap));
                }
            }
            q.push_back(val);
            Ok(())
        } else {
            Err("Failed to lock channel".to_string())
        }
    }

    pub fn recv(&self) -> Option<T> {
        if let Ok(mut q) = self.inner.lock() {
            q.pop_front()
        } else {
            None
        }
    }
}

pub struct ActorMailbox<M> {
    channel: Channel<M>,
}

impl<M> ActorMailbox<M> {
    pub fn new() -> Self {
        Self {
            channel: Channel::new(None),
        }
    }

    pub fn send(&self, msg: M) -> Result<(), String> {
        self.channel.send(msg)
    }

    pub fn process_all<F: FnMut(M)>(&self, mut handler: F) -> usize {
        let mut count = 0;
        while let Some(msg) = self.channel.recv() {
            handler(msg);
            count += 1;
        }
        count
    }
}
