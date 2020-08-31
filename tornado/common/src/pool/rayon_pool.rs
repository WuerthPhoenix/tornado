use crossbeam_channel::*;
use std::thread;
use std::sync::Arc;
use log::*;
use rayon::ThreadPool;
use crate::TornadoError;

pub fn start<F, M>(threads: usize, channel_size: usize, callback: Arc<F>) -> Result<Sender<M>, TornadoError>
    where
        M: Send + Sync + 'static,
        F: Fn(M) + Send + Sync + 'static,
{
    rayon::ThreadPoolBuilder::new().num_threads(threads).build()
        .map_err(|err| TornadoError::ConfigurationError {
            message: format!("{:?}", err)
        }).map(|pool| start_with_pool(pool, channel_size, callback))
}

pub fn start_with_pool<F, M>(thread_pool: ThreadPool, channel_size: usize, callback: Arc<F>) -> Sender<M>
    where
        M: Send + Sync + 'static,
        F: Fn(M) + Send + Sync + 'static,
{
    let (sender, receiver) = bounded(channel_size);

    thread::spawn(move || {
        loop {
            match receiver.recv() {
                Ok(message) => {
                    let callback_clone = callback.clone();
                    thread_pool.spawn( move || {
                        callback_clone(message);
                    });
                },
                Err(err) => {
                    error!("Error while receiving Message from channel. The receiver will be stopped. Error: {:?}", err);
                    break;
                }
            }

        }
    });

    sender
}