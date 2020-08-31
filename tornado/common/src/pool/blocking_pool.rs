use crossbeam_channel::*;
use std::thread;
use std::sync::Arc;
use log::*;

pub fn start<F, M>(threads: usize, channel_size: usize, callback: Arc<F>) -> Sender<M>
    where
        M: Send + Sync + 'static,
        F: Fn(M) + Send + Sync + 'static,
{
    let (sender, receiver) = bounded(channel_size);

    for _ in 0..threads {
        let receiver_clone = receiver.clone();
        let callback_clone = callback.clone();
        thread::spawn(move || {
            loop {
                match receiver_clone.recv() {
                    Ok(message) => {
                        callback_clone(message);
                    },
                    Err(err) => {
                        error!("Error while receiving Message from channel. Error: {:?}", err);
                    }
                }

            }
        });
    }

    sender
}