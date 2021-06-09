use crate::command::Command;
use std::marker::PhantomData;
use std::rc::Rc;
use tracing_futures::Instrument;

/// A Command that spans the internal Command execution to another light thread
/// without waiting for its completion
pub struct SpawnCommand<I: 'static, O, W: 'static + Command<I, O>> {
    command: Rc<W>,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<I: 'static, O, W: 'static + Command<I, O>> SpawnCommand<I, O, W> {
    pub fn new(command: Rc<W>) -> Self {
        Self { command, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<I: 'static, O, W: 'static + Command<I, O>> Command<I, ()> for SpawnCommand<I, O, W> {
    async fn execute(&self, message: I) {
        let span = tracing::Span::current();
        let command = self.command.clone();
        actix::spawn(async move {
            command.execute(message).await;
        }.instrument(span));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::callback::CallbackCommand;
    use async_channel::unbounded;
    use tokio::time;

    #[actix_rt::test]
    async fn should_execute_in_a_new_green_thread() {
        // Arrange
        let calls: i64 = 100;
        let sleep_millis: i64 = 100;
        let now_millis = chrono::Utc::now().timestamp_millis();

        let (exec_tx, exec_rx) = unbounded();

        let sender = SpawnCommand::new(Rc::new(CallbackCommand::new(move |message: String| {
            let exec_tx_clone = exec_tx.clone();
            async move {
                time::sleep_until(
                    time::Instant::now() + time::Duration::from_millis(sleep_millis as u64),
                )
                .await;
                let _result = exec_tx_clone.send(message).await;
            }
        })));

        // Act
        for i in 0..calls {
            let message = format!("hello {}", i);
            println!("send message: [{:?}]", message);
            sender.execute(message).await;
        }

        // Assert
        for _ in 0..calls {
            assert!(exec_rx.recv().await.is_ok());
        }

        let after_millis = chrono::Utc::now().timestamp_millis();
        assert!(now_millis <= after_millis + sleep_millis);
        assert!(after_millis < now_millis + (sleep_millis * calls));
    }
}
