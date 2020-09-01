use async_channel::*;
use log::*;
use std::sync::Arc;

/// Executes a blocking callback every time a message is sent to the returned Sender.
/// The callback is executed in parallel with a fixed max_parallel_executions factor.
/// If more messages then max_parallel_executions are sent, the exceeding messages are kept in a queue with fixed buffer_size.
pub fn start_async_runner<F, M, R>(
    max_parallel_executions: usize,
    buffer_size: usize,
    callback: Arc<F>,
) -> Sender<M>
where
    M: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(M) -> R,
    R: Send + Sync + 'static + futures::Future<Output = ()>,
{
    let (sender, receiver) = bounded(buffer_size);

    for _ in 0..max_parallel_executions {
        let callback = callback.clone();
        let receiver = receiver.clone();

        actix::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(message) => {
                        callback(message).await;
                    }
                    Err(err) => {
                        error!("Error Message received from channel. The receiver will be stopped. Err: {:?}", err);
                        break;
                    }
                }
            }
        });
    }

    sender
}

#[cfg(test)]
mod test {

    use super::*;
    use tokio::time;

    #[actix_rt::test]
    async fn should_execute_max_parallel_async_tasks() {
        // Arrange
        let threads = 5;
        let buffer_size = 10;

        let (exec_tx, exec_rx) = unbounded();

        let sender = start_async_runner(
            threads,
            buffer_size,
            Arc::new(move |message: String| {
                let exec_tx_clone = exec_tx.clone();
                async move {
                    println!("processing message: [{}]", message);
                    time::delay_until(time::Instant::now() + time::Duration::from_millis(100))
                        .await;
                    println!("end processing message: [{}]", message);

                    // Do not use 'unwrap' here; the threadpool could survive the test and execute this call when the receiver is dropped.
                    let _result = exec_tx_clone.send(()).await;
                }
            }),
        );

        // Act

        //  Send enough messages to fill the buffer
        for i in 0..(buffer_size + threads) {
            let message = format!("hello {}", i);
            println!("send message: [{}]", message);
            assert!(sender.send(message).await.is_ok());
        }

        // Assert

        // the buffer is full and all threads are blocked, it should fail
        assert!(sender.try_send(format!("hello world")).is_err());

        // wait for at least one message to be processed
        assert!(exec_rx.recv().await.is_ok());

        time::delay_until(time::Instant::now() + time::Duration::from_millis(100)).await;

        // Once one message was processed, we should be able to send a new message
        assert!(sender.try_send(format!("hello world")).is_ok());
    }
}
