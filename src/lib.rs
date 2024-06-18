//!...

use std::hash::Hash;
use std::sync::Arc;
use std::thread::{current, park_timeout, Thread, ThreadId};
use std::time::Duration;

use dashmap::DashMap;

/// ...
#[derive(Clone)]
pub struct Broadcast<K>(Arc<DashMap<K, ahash::HashMap<ThreadId, Thread>, ahash::RandomState>>);

impl<K: Clone + Eq + Hash> Broadcast<K> {
    /// ...
    pub fn new() -> Self {
        let fast_hash = ahash::RandomState::new(); // ~20% faster than std
        Broadcast(Arc::new(DashMap::with_hasher(fast_hash)))
    }

    /// ...
    pub fn notify(&self, key: &K) {
        if let Some((_, waiters)) = self.0.remove(key) {
            for (_, thread) in waiters {
                thread.unpark();
            }
        }
    }

    /// ..
    pub fn wait(&self, key: K) {
        self.wait_while(key, Duration::MAX, || true);
    }

    /// ...
    pub fn wait_while(&self, key: K, check_every: Duration, condition: impl Fn() -> bool) {
        let thread_handle = current();
        let thread_id = thread_handle.id();

        {
            // scoped to prevent dashmap deadlock
            let mut waiters = self.0.entry(key.clone()).or_default();
            waiters.insert(thread_id, thread_handle);
        }

        loop {
            park_timeout(check_every);
            
            let is_notified = match self.0.get(&key) {
                Some(waiters) => !waiters.contains_key(&thread_id),
                None => false,
            };
            
            if is_notified || !condition() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    const A_KEY: i32 = 42;
    const OTHER_KEY: i32 = 123;
    const SOME_TIME: Duration = Duration::from_millis(10);

    #[test]
    fn waits_for_notification() {
        // given
        let broadcast = Broadcast::new();

        thread::scope(|s| {
            // when
            s.spawn(|| {
                broadcast.notify(&OTHER_KEY); // ignores

                thread::sleep(SOME_TIME);
                broadcast.notify(&A_KEY);
            });

            let before = Instant::now();
            broadcast.wait(A_KEY);

            // then
            assert!(before.elapsed() > SOME_TIME);
        });
    }

    #[test]
    fn handles_spurious_wakes() {
        // given
        let broadcast = Broadcast::new();
        let thread_handle = current();

        // when
        thread::scope(|s| {
            thread_handle.unpark(); // before waiting

            s.spawn(|| {
                thread::sleep(SOME_TIME / 2);
                thread_handle.unpark(); // while waiting

                thread::sleep(SOME_TIME / 2);
                broadcast.notify(&A_KEY);
            });

            let before = Instant::now();
            broadcast.wait(A_KEY);

            // then
            assert!(before.elapsed() > SOME_TIME);
        });
    }

    #[test]
    fn waits_until_predicate_fails() {
        // given
        let broadcast = Broadcast::new();

        // when
        let before = Instant::now();
        broadcast.wait_while(A_KEY, SOME_TIME / 10, || before.elapsed() < SOME_TIME);

        // then
        assert!(before.elapsed() > SOME_TIME);
    }

    #[test]
    fn generic_key() {
        Broadcast::new().notify(&());
        Broadcast::new().notify(&123);
        Broadcast::new().notify(b"hello");
    }

    #[test]
    fn implements_traits() {
        fn smoke(_: impl Clone + Send + Sync) {}
        smoke(Broadcast::<()>::new());
    }
}
