use std::{
    sync::{atomic::AtomicBool, Arc},
    task::Waker,
};

use futures::task::AtomicWaker;

pub struct Entry<V> {
    pub value: Option<V>,
    pub state: Arc<EntryState>,
    // TODO: Make sure this is actually a thing we use to clean up,
    // or maybe just punt it
    pub ref_count: Arc<()>,
}

impl<V: Clone> Clone for Entry<V> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            ref_count: self.ref_count.clone(),
            state: self.state.clone(),
        }
    }
}

#[derive(Default)]
pub struct EntryState {
    is_set: AtomicBool,
    waker: AtomicWaker,
}

impl EntryState {
    pub fn is_set(&self) -> bool {
        self.is_set.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn register(&self, waker: &Waker) {
        self.waker.register(waker);
    }

    pub fn wake(&self) {
        self.is_set
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.waker.wake();
    }
}
