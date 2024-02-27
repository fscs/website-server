use std::ops::Deref;
use async_std::sync::RwLockReadGuard;
use std::time::{Duration, SystemTime};
use async_std::sync::RwLock;

pub(crate) struct TimedCache<T: Sync> {
    data_last_updated: RwLock<DataLastUpdate<T>>,
    generator: Box<dyn Fn() -> T + Sync> ,
    duration: Duration
}

struct ReadWrapper<'a, T> (
    RwLockReadGuard<'a, DataLastUpdate<T>>
);

impl<'a, T :  'a> Deref for ReadWrapper<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}
struct DataLastUpdate<T> { data: T,  last_updated: SystemTime }

impl<T: Sync> TimedCache<T> {

    pub(crate) fn with_generator<FN: Fn() -> T + 'static + Sync>(generator: FN, duration: Duration) -> Self {
        TimedCache {
            data_last_updated: RwLock::new(DataLastUpdate {
                data: generator(),
                last_updated: SystemTime::now()}),
            generator: Box::new(generator),
            duration
        }
    }

    pub(crate) async fn get(&self) -> impl Deref<Target = T> + '_ {
        let data = self.data_last_updated.read().await;
        if (*data).last_updated + self.duration > SystemTime::now() {
            ReadWrapper(data)
        } else {
            drop(data);
            {
                let data = (*self.generator)(); //Blocks for now

                let mut write = self.data_last_updated.write().await;
                write.data = data;
                write.last_updated = SystemTime::now();
            }
            let data = self.data_last_updated.read().await;
            ReadWrapper(data)
        }
    }

}