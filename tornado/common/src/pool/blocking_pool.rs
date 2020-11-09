use crate::TornadoError;

use crate::pool::{ReplyRequest, Runner, Sender};
use async_channel::{bounded, unbounded};
use log::*;
use std::sync::Arc;
use std::thread;

/// Executes a blocking callback every time a message is sent to the returned Sender.
/// The callback is executed in parallel with a fixed max_parallel_executions factor.
/// If more messages than max_parallel_executions are sent, the exceeding messages are kept in a queue with fixed buffer_size.
pub fn start_blocking_runner<F, M, R, Run>(
    max_parallel_executions: usize,
    buffer_size: usize,
    factory: F,
) -> Result<Sender<M, R>, TornadoError>
where
    M: Send + Sync + 'static,
    F: Fn() -> Run,
    R: Send + Sync + 'static,
    Run: Send + 'static + Runner<M, R>,
{
    let (sender, receiver) = bounded::<ReplyRequest<M, R>>(buffer_size);

    for _ in 0..max_parallel_executions {
        let receiver = receiver.clone();
        let runner = factory();

        actix::spawn(async move {
            let (completion_tx, completion_rx) = unbounded();
            let mut runner = runner;
            loop {
                match receiver.recv().await {
                    Ok(message) => {
                        let completion_tx = completion_tx.clone();

                        thread::spawn( move || {

                            let response = runner.execute(message.msg);

                            if let Some(responder) = message.responder {
                                if let Err(err) = responder.try_send(response) {
                                    error!("Pool executor cannot send the response message. Err: {:?}", err);
                                };
                            }

                            if let Err(err) = completion_tx.try_send(runner) {
                                error!("Pool executor cannot send the completion message. The executor will not process messages anymore. Err: {:?}", err);
                            };
                        });
                        match completion_rx.recv().await {
                            Ok(run) => {
                                runner = run;
                            }
                            Err(err) => {
                                error!("Pool executor cannot receive the completion message. The executor will not process messages anymore. Err: {:?}", err);
                                break;
                            }
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::time;

    #[actix_rt::test]
    async fn should_execute_max_parallel_blocking_tasks() {
        // Arrange
        let threads = 5;
        let buffer_size = 10;

        let (exec_tx, exec_rx) = unbounded();

        let sender = start_blocking_runner(threads, buffer_size, move || {
            let exec_tx = exec_tx.clone();
            TestRunner {
                callback: Arc::new(move |message: String| {
                    println!("processing message: [{}]", message);
                    std::thread::sleep(Duration::from_millis(100));
                    println!("end processing message: [{}]", message);

                    // Do not use 'unwrap' here; the threadpool could survive the test and execute this call when the receiver is dropped.
                    let _result = exec_tx.try_send(());
                }),
            }
        })
        .unwrap();

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

        let sender = start_blocking_runner(threads, buffer_size, move || TestRunner {
            callback: Arc::new(move |message: String| {
                println!("processing message: [{}]", message);
                std::thread::sleep(Duration::from_millis(100));
                println!("end processing message: [{}]", message);
                message
            }),
        })
        .unwrap();

        let (exec_tx, exec_rx) = unbounded();
        let count = Arc::new(AtomicUsize::new(0));

        // Act
        for i in 0..3 {
            let exec_tx = exec_tx.clone();
            let sender = sender.clone();
            let count = count.clone();

            actix::spawn(async move {
                let message = format!("hello {}", i);
                let response = sender.send(message.clone()).await.unwrap();
                assert_eq!(message, response);
                count.fetch_add(1, Ordering::SeqCst);
                exec_tx.try_send(message).unwrap();
            })
        }

        let expected_messages = vec!["hello 0", "hello 1", "hello 2"];
        for _ in 0..3 {
            let response = exec_rx.recv().await.unwrap();
            assert!(expected_messages.contains(&response.as_str()));
        }

        assert_eq!(3, count.load(Ordering::SeqCst));
    }

    struct TestRunner<M, R> {
        pub callback: Arc<dyn Sync + Send + Fn(M) -> R>,
    }

    impl<M, R> Runner<M, R> for TestRunner<M, R> {
        fn execute(&mut self, msg: M) -> R {
            (self.callback)(msg)
        }
    }
}
