use crate::{StatelessExecutor, ExecutorError, StatefulExecutor};
use async_channel::{bounded, Sender};
use tornado_common_api::Action;
use tokio::sync::Semaphore;
use log::*;
use std::rc::Rc;

pub struct ReplyRequest {
    pub action: Rc<Action>,
    pub responder: async_channel::Sender<Result<(), ExecutorError>>,
}

/// An executor pool.
/// It allocates a fixed pool of StatefulExecutors with a max concurrent access factor.
pub struct StatefulExecutorPool {
    sender: Sender<ReplyRequest>,
}

impl StatefulExecutorPool {
    pub fn new<F: Fn() -> T, T: 'static + StatefulExecutor>(max_parallel_executions: usize, factory: F) -> Self {
        let (sender, receiver) = bounded::<ReplyRequest>(max_parallel_executions);

        for _ in 0..max_parallel_executions {
            let mut executor = factory();
            let receiver = receiver.clone();

            actix::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(message) => {
                            let response = executor.execute(message.action).await;
                            if let Err(err) = message.responder.try_send(response) {
                                error!(
                                    "StatefulExecutorPool cannot send the response message. Err: {:?}",
                                    err
                                );
                            };
                        }
                        Err(err) => {
                            error!("StatefulExecutorPool received error from channel. The receiver will be stopped. Err: {:?}", err);
                            break;
                        }
                    }
                }
            });
        }
        Self {
            sender
        }
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for StatefulExecutorPool {
    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        let (tx, rx) = async_channel::bounded(1);
        self.sender.send(ReplyRequest { action, responder: tx }).await.map_err(|err| {
            ExecutorError::SenderError { message: format!("Error sending message: {:?}", err) }
        })?;
        rx.recv().await.map_err(|err| ExecutorError::SenderError {
            message: format!("Error receiving message response: {:?}", err),
        })?
    }
}

/// An executor pool.
/// It allocates a fixed pool of StatelessExecutors with a max concurrent access factor.
pub struct StatelessExecutorPool<T: StatelessExecutor> {
    semaphore: Semaphore,
    executor: T
}

impl <T: StatelessExecutor> StatelessExecutorPool<T> {
    pub fn new(max_parallel_executions: usize, executor: T) -> Self {
        Self{
            semaphore: Semaphore::new(max_parallel_executions),
            executor
        }
    }
}

#[async_trait::async_trait(?Send)]
impl <T: StatelessExecutor> StatelessExecutor for StatelessExecutorPool<T> {

    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        let _guard = self.semaphore.acquire().await;
        self.executor.execute(action).await
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use async_channel::unbounded;
    use std::sync::atomic::{AtomicUsize, Ordering};
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

    /*
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

        // Assert
        let expected_messages = vec!["hello 0", "hello 1", "hello 2"];
        for _ in 0..3 {
            let response = exec_rx.recv().await.unwrap();
            assert!(expected_messages.contains(&response.as_str()));
        }

        assert_eq!(3, count.load(Ordering::SeqCst));
    }
         */
}
