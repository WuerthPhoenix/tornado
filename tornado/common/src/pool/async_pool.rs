use crate::pool::{ReplyRequest, Sender};
use async_channel::bounded;
use log::*;
use std::sync::Arc;

/// Executes a blocking callback every time a message is sent to the returned Sender.
/// The callback is executed in parallel with a fixed max_parallel_executions factor.
/// If more messages then max_parallel_executions are sent, the exceeding messages are kept in a queue with fixed buffer_size.
pub fn start_async_runner<F, Fut, M, R>(
    max_parallel_executions: usize,
    buffer_size: usize,
    callback: Arc<F>,
) -> Sender<M, R>
where
    M: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(M) -> Fut,
    Fut: Send + Sync + 'static + futures::Future<Output = R>,
    R: Send + Sync + 'static,
{
    let (sender, receiver) = bounded::<ReplyRequest<M, R>>(buffer_size);

    for _ in 0..max_parallel_executions {
        let callback = callback.clone();
        let receiver = receiver.clone();

        actix::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(message) => {
                        let response = callback(message.msg).await;

                        if let Some(responder) = message.responder {
                            if let Err(err) = responder.try_send(response) {
                                error!(
                                    "Pool executor cannot send the response message. Err: {:?}",
                                    err
                                );
                            };
                        }
                    }
                    Err(err) => {
                        error!("Error Message received from channel. The receiver will be stopped. Err: {:?}", err);
                        break;
                    }
                }
            }
        });
    }

    Sender::new(sender)
}

#[cfg(test)]
mod test {

    use super::*;
    use async_channel::unbounded;
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
            assert!(sender.try_send(message).is_ok());
            time::delay_until(time::Instant::now() + time::Duration::from_millis(1)).await;
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

    #[actix_rt::test]
    async fn should_send_and_wait_for_response() {
        // Arrange
        let threads = 5;
        let buffer_size = 10;

        let sender = start_async_runner(
            threads,
            buffer_size,
            Arc::new(move |message: String| async move {
                println!("processing message: [{}]", message);
                time::delay_until(time::Instant::now() + time::Duration::from_millis(100)).await;
                println!("end processing message: [{}]", message);
                message
            }),
        );

        let (exec_tx, exec_rx) = unbounded();

        // Act
        for i in 0..3 {
            let exec_tx = exec_tx.clone();
            let sender = sender.clone();
            actix::spawn(async move {
                let message = format!("hello {}", i);
                let response = sender.send(message.clone()).await.unwrap();
                assert_eq!(message, response);
                exec_tx.try_send(message).unwrap();
            })
        }

        let expected_messages = vec!["hello 0", "hello 1", "hello 2"];
        for _ in 0..3 {
            let response = exec_rx.recv().await.unwrap();
            assert!(expected_messages.contains(&response.as_str()));
        }
    }
}
