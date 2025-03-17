use dashmap::DashMap;
use entry::{Entry, EntryState};
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

mod entry;

pub struct SubscriberMap<K, V> {
    inner: Arc<DashMap<K, Entry<V>>>,
}

impl<K: Eq + Hash, V> Default for SubscriberMap<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

#[pin_project::pin_project]
pub struct Subscription<K, V> {
    key: K,
    state: Arc<EntryState>,
    map: Arc<DashMap<K, Entry<V>>>,
}

impl<K: Eq + Hash, V: Clone> Future for Subscription<K, V> {
    type Output = V;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if this.state.is_set() {
            let v = this
                .map
                .get(&this.key)
                .expect("must be here")
                .value()
                .value
                .clone()
                .expect("must be here");
            return Poll::Ready(v);
        }

        this.state.register(cx.waker());

        if this.state.is_set() {
            let v = this
                .map
                .get(&this.key)
                .expect("must be here")
                .value()
                .value
                .clone()
                .expect("must be here");
            return Poll::Ready(v);
        } else {
            Poll::Pending
        }
    }
}

impl<K, V> SubscriberMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn subscribe(&self, key: K) -> Subscription<K, V> {
        use dashmap::Entry::*;
        match self.inner.entry(key.clone()) {
            Occupied(occupied_entry) => {
                let e = occupied_entry.get();
                let state = e.state.clone();
                Subscription {
                    key,
                    state,
                    map: self.inner.clone(),
                }
            }
            Vacant(vacant_entry) => {
                let state = Arc::new(EntryState::default());
                let e = Entry {
                    value: None,
                    state: state.clone(),
                    ref_count: Arc::new(()),
                };

                vacant_entry.insert(e);
                Subscription {
                    key,
                    state,
                    map: self.inner.clone(),
                }
            }
        }
    }

    pub fn insert(&self, key: K, value: V) {
        use dashmap::Entry::*;
        match self.inner.entry(key.clone()) {
            Occupied(mut occupied) => {
                let entry = occupied.get_mut();
                if entry.value.is_some() {
                    panic!("I'm not sure how I want to handle this case, maybe only override if ref count is 0/1?");
                }
                entry.value = Some(value);
                entry.state.wake();
            }
            Vacant(vacant) => {
                let state = Arc::new(EntryState::default());
                let e = Entry {
                    value: Some(value),
                    state: state.clone(),
                    ref_count: Arc::new(()),
                };

                vacant.insert(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_test() {
        let map = SubscriberMap::default();
        println!("Subscribing to 'hello'");
        let s1 = map.subscribe("hello".to_string());
        let h = tokio::task::spawn(async move {
            s1.await;
            println!("got my message");
        });

        println!("writing to map at 'hello'");
        map.insert("hello".to_string(), 5);

        _ = h.await;
    }
}
