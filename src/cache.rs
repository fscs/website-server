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

    pub(crate) async fn get(&self) -> impl Deref<Target = T> + '_ {
        let data = self.data_last_updated.read().await;
        if (&data)
            .iter()
            .any(|d| d.last_updated + self.duration < SystemTime::now())
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
