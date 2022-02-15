use crate::command::{Command, CommandMut};
use async_channel::{bounded, Sender};
use log::*;
use std::marker::PhantomData;
use tokio::sync::Semaphore;
use tornado_executor_common::ExecutorError;
use tracing::Span;
use tracing_futures::Instrument;

pub struct ReplyRequest<I, O> {
    pub span: Span,
    pub message: I,
    pub responder: async_channel::Sender<O>,
}

/// A Command pool.
/// It allows a max concurrent number of accesses to the internal Command.
pub struct CommandPool<I, O, T: Command<I, O>> {
    semaphore: Semaphore,
    command: T,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<I, O, T: Command<I, O>> CommandPool<I, O, T> {
    pub fn new(max_parallel_executions: usize, command: T) -> Self {
        Self {
            semaphore: Semaphore::new(max_parallel_executions),
            command,
            phantom_i: PhantomData,
            phantom_o: PhantomData,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<I, O, T: Command<I, O>> Command<I, O> for CommandPool<I, O, T> {
    async fn execute(&self, message: I) -> O {
        let _guard = self.semaphore.acquire().await;
        self.command.execute(message).await
    }
}

/// A CommandMut pool.
/// It allocates a fixed pool of CommandMut with a max concurrent access factor.
pub struct CommandMutPool<I: 'static, O: 'static> {
    sender: Sender<ReplyRequest<I, Result<O, ExecutorError>>>,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<I: 'static, O: 'static> CommandMutPool<I, O> {
    pub fn new<F: Fn() -> T, T: 'static + CommandMut<I, Result<O, ExecutorError>>>(
        max_parallel_executions: usize,
        factory: F,
    ) -> Self {
        let (sender, receiver) =
            bounded::<ReplyRequest<I, Result<O, ExecutorError>>>(max_parallel_executions);

        for _ in 0..max_parallel_executions {
            let mut command = factory();
            let receiver = receiver.clone();

            actix::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(message) => {
                            let _entered_span = message.span.enter();
                            let response = command
                                .execute(message.message)
                                .instrument(message.span.clone())
                                .await;
                            if let Err(err) = message.responder.try_send(response) {
                                error!(
                                    "CommandMutPool cannot send the response message. Err: {:?}",
                                    err
                                );
                            };
                        }
                        Err(err) => {
                            error!("CommandMutPool received error from channel. The receiver will be stopped. Err: {:?}", err);
                            break;
                        }
                    }
                }
            });
        }
        Self { sender, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<I: 'static, O: 'static> Command<I, Result<O, ExecutorError>> for CommandMutPool<I, O> {
    async fn execute(&self, message: I) -> Result<O, ExecutorError> {
        let (tx, rx) = async_channel::bounded(1);
        let span = tracing::Span::current();
        self.sender.send(ReplyRequest { span, message, responder: tx }).await.map_err(|err| {
            ExecutorError::SenderError { message: format!("Error sending message: {:?}", err) }
        })?;
        rx.recv().await.map_err(|err| ExecutorError::SenderError {
            message: format!("Error receiving message response: {:?}", err),
        })?
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::command::callback::{CallbackCommand, CallbackCommandMut};
    use crate::TornadoError;
    use async_channel::unbounded;
    use std::sync::Arc;
    use tokio::time;
    use tornado_common_api::Action;

    #[actix_rt::test]
    async fn stateful_pool_should_execute_max_parallel_async_tasks() {
        // Arrange
        let threads = 5;

        let (exec_tx, exec_rx) = unbounded();

        let sender = Arc::new(CommandMutPool::new(threads, move || {
            let exec_tx_clone = exec_tx.clone();
            CallbackCommandMut::new(move |action: Arc<Action>| {
                let exec_tx_clone = exec_tx_clone.clone();
                async move {
                    println!("processing message: [{:?}]", action);
                    time::sleep_until(time::Instant::now() + time::Duration::from_millis(100))
                        .await;
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
                assert!(sender.execute(Arc::new(message)).await.is_ok());
                // There should never be more messages in the queue than available threads
                assert!(exec_rx.len() <= threads);
                time::sleep_until(time::Instant::now() + time::Duration::from_millis(1)).await;
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
        let sender = Arc::new(CommandPool::new(
            threads,
            CallbackCommand::<_, _, _, Result<(), TornadoError>>::new(
                move |action: Arc<Action>| {
                    let exec_tx_clone = exec_tx_clone.clone();
                    async move {
                        println!("processing message: [{:?}]", action);
                        time::sleep_until(time::Instant::now() + time::Duration::from_millis(100))
                            .await;
                        println!("end processing message: [{:?}]", action);

                        // Do not use 'unwrap' here; the threadpool could survive the test and execute this call when the receiver is dropped.
                        let _result = exec_tx_clone.send(()).await;
                        Ok(())
                    }
                },
            ),
        ));

        // Act
        let loops = 10;

        for i in 0..(loops * threads) {
            let exec_rx = exec_rx.clone();
            let sender = sender.clone();
            actix::spawn(async move {
                let message = Action::new(&format!("hello {}", i));
                println!("send message: [{:?}]", message);
                assert!(sender.execute(Arc::new(message)).await.is_ok());
                // There should never be more messages in the queue than available threads
                assert!(exec_rx.len() <= threads);
                time::sleep_until(time::Instant::now() + time::Duration::from_millis(1)).await;
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

        let sender = Arc::new(CommandMutPool::new(threads, move || {
            CallbackCommandMut::new(move |action: Arc<Action>| async move {
                println!("processing message: [{:?}]", action);
                time::sleep_until(time::Instant::now() + time::Duration::from_millis(10)).await;
                println!("end processing message: [{:?}]", action);
                if action.id.contains("err") {
                    Err(ExecutorError::SenderError { message: action.id.to_owned() })
                } else {
                    Ok(action.id.to_owned())
                }
            })
        }));

        // Act
        for i in 0..100 {
            if i % 2 == 0 {
                let message = format!("hello {}", i);
                let result = sender.execute(Action::new(&message).into()).await;
                match result {
                    Ok(result_message) => assert_eq!(result_message, message),
                    _ => assert!(false),
                }
            } else {
                let message = format!("err {}", i);
                let result = sender.execute(Action::new(&message).into()).await;
                match result {
                    Err(ExecutorError::SenderError { message: err_message }) => {
                        assert_eq!(err_message, message)
                    }
                    _ => assert!(false),
                }
            }
        }
    }

    #[actix_rt::test]
    async fn stateless_pool_should_send_and_wait_for_response() {
        // Arrange
        let threads = 5;

        let sender = Arc::new(CommandPool::new(
            threads,
            CallbackCommand::new(move |action: Arc<Action>| async move {
                println!("processing message: [{:?}]", action);
                time::sleep_until(time::Instant::now() + time::Duration::from_millis(10)).await;
                println!("end processing message: [{:?}]", action);
                if action.id.contains("err") {
                    Err(TornadoError::SenderError { message: action.id.to_owned() })
                } else {
                    Ok(action.id.to_owned())
                }
            }),
        ));

        // Act
        for i in 0..100 {
            if i % 2 == 0 {
                let message = format!("hello {}", i);
                let result = sender.execute(Action::new(&message).into()).await;
                match result {
                    Ok(result_message) => assert_eq!(result_message, message),
                    _ => assert!(false),
                }
            } else {
                let message = format!("err {}", i);
                let result = sender.execute(Action::new(&message).into()).await;
                match result {
                    Err(TornadoError::SenderError { message: err_message }) => {
                        assert_eq!(err_message, message)
                    }
                    _ => assert!(false),
                }
            }
        }
    }
}
