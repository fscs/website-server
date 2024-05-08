use async_std::sync::RwLock;
use async_std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

pub(crate) struct TimedCache<T: Sync> {
    data_last_updated: RwLock<Option<DataLastUpdate<T>>>,
    generator: Box<dyn Fn() -> Pin<Box<dyn Future<Output = T>>> + 'static + Sync>,
    duration: Duration,
}

struct ReadWrapper<'a, T>(
    // Option mau NEVER be NONE
    RwLockReadGuard<'a, Option<DataLastUpdate<T>>>,
);

impl<'a, T: 'a> Deref for ReadWrapper<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.as_ref().unwrap().data
    }
}

struct DataLastUpdate<T> {
    data: T,
    last_updated: SystemTime,
}

impl<T: Sync> TimedCache<T> {
    /// Create a TimedCache that generates a Value from the given Function.
    /// The Function is normally not `pure`, it is expected, that the Output can change.
    /// The Duration should be a Time in which, the Output is expected to not change.
    pub(crate) fn with_generator<FN: Fn() -> Pin<Box<dyn Future<Output = T>>> + 'static + Sync>(
        generator: FN,
        duration: Duration,
    ) -> Self {
        TimedCache {
            data_last_updated: RwLock::new(None),
            generator: Box::new(generator),
            duration,
        }
    }

    /// Get the Value of the TimedCache.
    ///
    /// Executes the generator function, if the last time it ran is more than Duration ago.
    ///
    ///
    /// ## Usage
    /// ```rust
    /// # tokio_test::block_on(async {
    /// lazy_static! {
    ///   static ref CACHE: TimedCache<u64> TimedCache::with_generator(
    ///     Box::pin(|| async {0}),
    ///     std::time::Duration::from_secs(60));
    /// }
    ///
    /// assert_eq!(CACHE.get.await, 0)
    /// # })
    /// ```
    pub(crate) async fn get(&self) -> impl Deref<Target = T> + '_ {
        let data = self.data_last_updated.read().await;
        if (data)
            .iter()
            .any(|d| d.last_updated + self.duration > SystemTime::now())
        {
            ReadWrapper(data)
        } else {
            drop(data);
            let mut write = self.data_last_updated.write().await;
            // Write only needed if not already updated
            if (*write).is_none()
                || (*write)
                    .iter()
                    .any(|d| d.last_updated + self.duration < SystemTime::now())
            {
                let data = (*self.generator)().await;

                *write = Some(DataLastUpdate {
                    data,
                    last_updated: SystemTime::now(),
                });
            }
            let data = RwLockWriteGuard::downgrade(write);
            ReadWrapper(data)
        }
    }
}

impl<T: Sync, E: Sync> TimedCache<Result<T, E>> {
    pub(crate) async fn try_get(&self) -> impl Deref<Target = Result<T, E>> + '_ {
        let data = self.data_last_updated.read().await;
        if data
            .iter()
            .filter(|d| d.data.is_ok())
            .any(|d| d.last_updated + self.duration > SystemTime::now())
        {
            ReadWrapper(data)
        } else {
            drop(data);
            let mut write = self.data_last_updated.write().await;
            // Write only needed if not already updated
            if (*write).is_none()
                || (*write)
                    .iter()
                    .any(|d| d.last_updated + self.duration < SystemTime::now() || d.data.is_err())
            {
                let data = (*self.generator)().await;

                *write = Some(DataLastUpdate {
                    data,
                    last_updated: SystemTime::now(),
                });
            }
            let data = RwLockWriteGuard::downgrade(write);
            ReadWrapper(data)
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use lazy_static::lazy_static;
    use std::sync::Mutex;

    use crate::cache::TimedCache;

    #[tokio::test]
    async fn test_cache() {
        lazy_static! {
            static ref CACHE: TimedCache<u64> = TimedCache::with_generator(
                || Box::pin(async { 0 }),
                std::time::Duration::from_secs(60)
            );
        }

        assert_eq!(*CACHE.get().await, 0)
    }

    #[tokio::test]
    async fn test_retry() {
        let x = Arc::new(Mutex::new(0));
        let a = x.clone();

        let cache = TimedCache::with_generator(
            move || {
                let x = a.clone();
                Box::pin(async move {
                    let mut x = x.lock().unwrap();
                    *x = *x + 1;
                    if *x == 1 {
                        Err(1)
                    } else {
                        Ok(2)
                    }
                })
            },
            std::time::Duration::from_secs(60),
        );

        assert!(*cache.try_get().await == Err(1));
        assert!(*cache.try_get().await == Ok(2))
    }
}
