use crate::{ExecutorError, StatefulExecutor, StatelessExecutor};
use async_channel::{bounded, Sender};
use log::*;
use std::rc::Rc;
use tokio::sync::Semaphore;
use tornado_common_api::Action;

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
    pub fn new<F: Fn() -> T, T: 'static + StatefulExecutor>(
        max_parallel_executions: usize,
        factory: F,
    ) -> Self {
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
        Self { sender }
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
    executor: T,
}

impl<T: StatelessExecutor> StatelessExecutorPool<T> {
    pub fn new(max_parallel_executions: usize, executor: T) -> Self {
        Self { semaphore: Semaphore::new(max_parallel_executions), executor }
    }
}

#[async_trait::async_trait(?Send)]
impl<T: StatelessExecutor> StatelessExecutor for StatelessExecutorPool<T> {
    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        let _guard = self.semaphore.acquire().await;
        self.executor.execute(action).await
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::callback::{CallbackStatefulExecutor, CallbackStatelessExecutor};
    use async_channel::unbounded;
    use std::sync::Arc;
    use tokio::time;

    #[actix_rt::test]
    async fn stateful_pool_should_execute_max_parallel_async_tasks() {
        // Arrange
        let threads = 5;

        let (exec_tx, exec_rx) = unbounded();

        let sender = Arc::new(StatefulExecutorPool::new(threads, move || {
            let exec_tx_clone = exec_tx.clone();
            CallbackStatefulExecutor::new(move |action: Rc<Action>| {
                let exec_tx_clone = exec_tx_clone.clone();
                async move {
                    println!("processing message: [{:?}]", action);
                    time::delay_until(time::Instant::now() + time::Duration::from_millis(100)).await;
                    println!("end processing message: [{:?}]", action);

                    // Do not use 'unwrap' here; the threadpool could survive the test and execute this call when the receiver is dropped.
                    let _result = exec_tx_clone.send(()).await;
                    Ok(())
                }
            })
        }));

        // Act
        let loops = 10;

        for i in 0..(loops * threads) {
            let exec_rx = exec_rx.clone();
            let sender = sender.clone();
            actix::spawn(async move {
                let message = Action::new(&format!("hello {}", i));
                println!("send message: [{:?}]", message);
                assert!(sender.execute(Rc::new(message)).await.is_ok());
                // There should never be more messages in the queue than available threads
                assert!(exec_rx.len() <= threads);
                time::delay_until(time::Instant::now() + time::Duration::from_millis(1)).await;
            });
        }

        // Assert
        for _ in 0..(loops * threads) {
            assert!(exec_rx.recv().await.is_ok());
            // There should never be more messages in the queue than available threads
            assert!(exec_rx.len() <= threads);
        }

    }

    #[actix_rt::test]
    async fn stateless_pool_should_execute_max_parallel_async_tasks() {
        // Arrange
        let threads = 5;

        let (exec_tx, exec_rx) = unbounded();

        let exec_tx_clone = exec_tx.clone();
        let sender = Arc::new(StatelessExecutorPool::new(threads,
                                                         CallbackStatelessExecutor::new(move |action: Rc<Action>| {
                let exec_tx_clone = exec_tx_clone.clone();
                async move {
                    println!("processing message: [{:?}]", action);
                    time::delay_until(time::Instant::now() + time::Duration::from_millis(100)).await;
                    println!("end processing message: [{:?}]", action);

                    // Do not use 'unwrap' here; the threadpool could survive the test and execute this call when the receiver is dropped.
                    let _result = exec_tx_clone.send(()).await;
                    Ok(())
                }
        })));

        // Act
        let loops = 10;

        for i in 0..(loops * threads) {
            let exec_rx = exec_rx.clone();
            let sender = sender.clone();
            actix::spawn(async move {
                let message = Action::new(&format!("hello {}", i));
                println!("send message: [{:?}]", message);
                assert!(sender.execute(Rc::new(message)).await.is_ok());
                // There should never be more messages in the queue than available threads
                assert!(exec_rx.len() <= threads);
                time::delay_until(time::Instant::now() + time::Duration::from_millis(1)).await;
            });
        }

        // Assert
        for _ in 0..(loops * threads) {
            assert!(exec_rx.recv().await.is_ok());
            // There should never be more messages in the queue than available threads
            assert!(exec_rx.len() <= threads);
        }

    }

    #[actix_rt::test]
    async fn stateful_pool_should_send_and_wait_for_response() {
        // Arrange
        let threads = 5;

        let sender = Arc::new(StatefulExecutorPool::new(threads, move || {
            CallbackStatefulExecutor::new(move |action: Rc<Action>| {
                async move {
                    println!("processing message: [{:?}]", action);
                    time::delay_until(time::Instant::now() + time::Duration::from_millis(100)).await;
                    println!("end processing message: [{:?}]", action);
                    if action.id.eq("err") {
                        Err(ExecutorError::SenderError {
                            message: "".to_owned()
                        })
                    } else {
                       Ok(())
                    }
                }
            })
        }));

        // Act
        for i in 0..3 {
            if i % 2 == 0  {
                assert!(sender.execute(Action::new(&format!("hello {}", i)).into()).await.is_ok());
            } else {
                assert!(sender.execute(Action::new("err").into()).await.is_err());
            }
        }
    }

    #[actix_rt::test]
    async fn stateless_pool_should_send_and_wait_for_response() {
        // Arrange
        let threads = 5;

        let sender = Arc::new(StatelessExecutorPool::new(threads,
            CallbackStatelessExecutor::new(move |action: Rc<Action>| {
                async move {
                    println!("processing message: [{:?}]", action);
                    time::delay_until(time::Instant::now() + time::Duration::from_millis(100)).await;
                    println!("end processing message: [{:?}]", action);
                    if action.id.eq("err") {
                        Err(ExecutorError::SenderError {
                            message: "".to_owned()
                        })
                    } else {
                        Ok(())
                    }
                }
        })));

        // Act
        for i in 0..3 {
            if i % 2 == 0  {
                assert!(sender.execute(Action::new(&format!("hello {}", i)).into()).await.is_ok());
            } else {
                assert!(sender.execute(Action::new("err").into()).await.is_err());
            }
        }
    }
}
