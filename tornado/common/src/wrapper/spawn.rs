use crate::wrapper::Wrapper;
use std::marker::PhantomData;
use std::rc::Rc;

/// A wrapper that spans the internal wrapper execution to another light thread
/// without waiting for its execution to complete
pub struct SpawnWrapper<I: 'static, O, W: 'static + Wrapper<I, O>> {
    executor: Rc<W>,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<I: 'static, O, W: 'static + Wrapper<I, O>> SpawnWrapper<I, O, W> {
    pub fn new(executor: Rc<W>) -> Self {
        Self { executor, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<I: 'static, O, W: 'static + Wrapper<I, O>> Wrapper<I, ()> for SpawnWrapper<I, O, W> {
    async fn execute(&self, message: I) {
        let executor = self.executor.clone();
        actix::spawn(async move {
            executor.execute(message).await;
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::wrapper::callback::CallbackWrapper;
    use async_channel::unbounded;
    use tokio::time;

    #[actix_rt::test]
    async fn should_execute_in_a_new_green_thread() {
        // Arrange
        let calls: i64 = 100;
        let sleep_millis: i64 = 100;
        let now_millis = chrono::Utc::now().timestamp_millis();

        let (exec_tx, exec_rx) = unbounded();

        let sender = SpawnWrapper::new(Rc::new(CallbackWrapper::new(move |message: String| {
            let exec_tx_clone = exec_tx.clone();
            async move {
                time::delay_until(
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
