use codex_protocol::ThreadId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::OwnedMutexGuard;

/// Short-lived locks that serialize durable lifecycle finalization for one thread identity.
#[derive(Default)]
pub(super) struct ThreadLifecycleLocks {
    locks: Mutex<HashMap<ThreadId, Weak<AsyncMutex<()>>>>,
}

pub(crate) struct ThreadLifecycleGuard {
    thread_id: ThreadId,
    _guard: OwnedMutexGuard<()>,
}

impl ThreadLifecycleLocks {
    pub(super) async fn lock(&self, thread_id: ThreadId) -> ThreadLifecycleGuard {
        let lock = {
            let mut locks = self
                .locks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            locks.retain(|_, lock| lock.strong_count() > 0);
            match locks.get(&thread_id).and_then(Weak::upgrade) {
                Some(lock) => lock,
                None => {
                    let lock = Arc::new(AsyncMutex::new(()));
                    locks.insert(thread_id, Arc::downgrade(&lock));
                    lock
                }
            }
        };
        ThreadLifecycleGuard {
            thread_id,
            _guard: lock.lock_owned().await,
        }
    }
}

impl ThreadLifecycleGuard {
    pub(crate) fn thread_id(&self) -> ThreadId {
        self.thread_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn locks_serialize_one_thread_without_blocking_other_threads() {
        let locks = ThreadLifecycleLocks::default();
        let first_thread_id = ThreadId::new();
        let other_thread_id = ThreadId::new();
        let first_guard = locks.lock(first_thread_id).await;

        assert!(
            tokio::time::timeout(Duration::from_millis(20), locks.lock(first_thread_id))
                .await
                .is_err()
        );
        let other_guard = tokio::time::timeout(Duration::from_secs(1), locks.lock(other_thread_id))
            .await
            .expect("an unrelated thread identity should not block");

        assert_eq!(first_guard.thread_id(), first_thread_id);
        assert_eq!(other_guard.thread_id(), other_thread_id);
    }
}
