use actix::{Actor, Addr, Context, Handler, Message};
use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use log::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tornado_common::pool::Sender;
use tornado_executor_common::RetriableError;

/// Defines the strategy to apply in case of a failure.
/// This is applied, for example, when an action execution fails
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RetryStrategy {
    pub retry_policy: RetryPolicy,
    pub backoff_policy: BackoffPolicy,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            retry_policy: RetryPolicy::MaxRetries { retries: 20 },
            backoff_policy: BackoffPolicy::Exponential { ms: 1000, multiplier: 2 },
        }
    }
}

impl RetryStrategy {
    /// Returns whether a retry attempt should be performed and an optional backoff time
    pub fn should_retry(&self, failed_attempts: u32) -> (bool, Option<Duration>) {
        (
            self.retry_policy.should_retry(failed_attempts),
            self.backoff_policy.should_wait(failed_attempts),
        )
    }
}

// Defines the retry policy of a RetryStrategy
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum RetryPolicy {
    /// No Retry attempts defined
    None,
    /// The operation will be retried for a max number of times.
    MaxRetries { retries: u32 },
    /// The operation will be retried an infinite number of times.
    Infinite,
    // Timeout,
}

impl RetryPolicy {
    fn should_retry(&self, failed_attempts: u32) -> bool {
        if failed_attempts == 0 {
            true
        } else {
            match self {
                RetryPolicy::None => false,
                RetryPolicy::Infinite => true,
                RetryPolicy::MaxRetries { retries: attempts } => *attempts + 1 > failed_attempts,
            }
        }
    }
}

// Defines the backoff policy of a RetryStrategy
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum BackoffPolicy {
    /// No backoff, the retry will be attempted without waiting
    None,
    /// A fixed amount ot time will be waited between each retry attempt
    Fixed { ms: u32 },
    /// Permits to specify the amount of time between two consecutive retry attempts.
    /// The time to wait after 'i' retries is specified in the vector at position 'i'.
    /// If the number of retries is bigger than the vector length, then the last value in the vector is used.
    /// For example:
    /// ms = [111,222,333] -> It waits 111 ms after the first failure, 222 ms after the second failure and then 333 ms for all following failures.
    Variable { ms: Vec<u32> },
    /// Implementation of BackoffPolicy that increases the back off period for each retry attempt in a given set using the exponential function.
    Exponential {
        /// The period to sleep on the first backoff.
        ms: u32,
        // The multiplier to use to generate the next backoff interval from the last.
        multiplier: u64,
    },
}

impl BackoffPolicy {
    fn should_wait(&self, failed_attempts: u32) -> Option<Duration> {
        if failed_attempts == 0 {
            None
        } else {
            match self {
                BackoffPolicy::None => None,
                BackoffPolicy::Fixed { ms } => {
                    if *ms > 0 {
                        Some(Duration::from_millis(*ms as u64))
                    } else {
                        None
                    }
                }
                BackoffPolicy::Variable { ms } => {
                    let index = (failed_attempts - 1) as usize;
                    let option_wait_ms = if ms.len() > index { ms.get(index) } else { ms.last() };
                    match option_wait_ms {
                        Some(wait_ms) => {
                            if *wait_ms > 0 {
                                Some(Duration::from_millis(*wait_ms as u64))
                            } else {
                                None
                            }
                        }
                        None => None,
                    }
                }
                BackoffPolicy::Exponential { ms, multiplier } => {
                    if *ms > 0 {
                        let multiplier = multiplier.saturating_pow(failed_attempts - 1);
                        let wait_ms = multiplier.saturating_mul(*ms as u64);
                        Some(Duration::from_millis(wait_ms))
                    } else {
                        None
                    }
                }
            }
        }
    }
}

pub struct RetryActor<
    M: 'static + Send + Message<Result = Result<(), Err>> + Clone + Unpin + Debug,
    Err: 'static + Send + Unpin + RetriableError + Display + Clone,
> {
    executor_addr: Sender<M, Result<(), Err>>,
    retry_strategy: Arc<RetryStrategy>,
    phantom_m: PhantomData<M>,
    phantom_err: PhantomData<Err>,
}

impl<
        M: 'static + Send + Message<Result = Result<(), Err>> + Clone + Unpin + Debug,
        Err: 'static + Send + Unpin + RetriableError + Display + Clone,
    > Actor for RetryActor<M, Err>
{
    type Context = Context<Self>;
}

impl<
        M: 'static + Send + Message<Result = Result<(), Err>> + Clone + Unpin + Debug,
        Err: 'static + Send + Unpin + RetriableError + Display + Clone,
    > RetryActor<M, Err>
{
    pub fn start_new<F>(retry_strategy: Arc<RetryStrategy>, factory: F) -> Addr<Self>
    where
        F: FnOnce() -> Sender<M, Result<(), Err>>,
    {
        let executor_addr = factory();
        Self { retry_strategy, executor_addr, phantom_m: PhantomData, phantom_err: PhantomData }
            .start()
    }
}

impl<
        M: 'static + Send + Message<Result = Result<(), Err>> + Clone + Unpin + Debug,
        Err: 'static + Send + Unpin + RetriableError + Display + Clone,
    > Handler<M> for RetryActor<M, Err>
{
    type Result = Result<(), Err>;

    fn handle(&mut self, msg: M, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("RetryActor - received new message");

        let executor_addr = self.executor_addr.clone();
        let retry_strategy = self.retry_strategy.clone();

        actix::spawn(async move {
            let mut should_retry = true;
            let mut failed_attempts = 0;
            while should_retry {
                should_retry = false;
                let result = executor_addr.send(msg.clone()).await;
                match result {
                    Ok(response) => {
                        if let Err(err) = response {
                            if !err.can_retry() {
                                warn!("The failed message will not be retried as the error is not recoverable. Err: {}", err)
                            } else {
                                failed_attempts += 1;
                                let (new_should_retry, should_wait) =
                                    retry_strategy.should_retry(failed_attempts);
                                should_retry = new_should_retry;

                                if should_retry {
                                    debug!("The failed message will be reprocessed based on the current RetryPolicy. Failed attempts: {}. Message: {:?}", failed_attempts, msg);
                                    if let Some(delay_for) = should_wait {
                                        debug!("Wait for {:?} before retrying.", delay_for);
                                        actix::clock::delay_for(delay_for).await;
                                    }
                                } else {
                                    warn!("The failed message will not be retried any more in respect of the current RetryPolicy. Failed attempts: {}. Message: {:?}", failed_attempts, msg)
                                }
                            }
                        }
                    }
                    Err(e) => error!("MailboxError: {}", e),
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::executor::{ActionMessage, ExecutorRunner};
    use rand::Rng;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
    use tornado_common_api::Action;
    use tornado_executor_common::{Executor, ExecutorError};

    #[test]
    fn retry_policy_none_should_never_retry() {
        assert!(RetryPolicy::None.should_retry(0));
        assert!(!RetryPolicy::None.should_retry(1));
        assert!(!RetryPolicy::None.should_retry(10));
        assert!(!RetryPolicy::None.should_retry(100));
    }

    #[test]
    fn retry_policy_max_should_return_when_to_retry() {
        assert!(RetryPolicy::MaxRetries { retries: 0 }.should_retry(0));
        assert!(!RetryPolicy::MaxRetries { retries: 0 }.should_retry(1));
        assert!(!RetryPolicy::MaxRetries { retries: 0 }.should_retry(10));
        assert!(!RetryPolicy::MaxRetries { retries: 0 }.should_retry(100));

        assert!(RetryPolicy::MaxRetries { retries: 1 }.should_retry(0));
        assert!(RetryPolicy::MaxRetries { retries: 1 }.should_retry(1));
        assert!(!RetryPolicy::MaxRetries { retries: 1 }.should_retry(2));
        assert!(!RetryPolicy::MaxRetries { retries: 1 }.should_retry(10));
        assert!(!RetryPolicy::MaxRetries { retries: 1 }.should_retry(100));

        assert!(RetryPolicy::MaxRetries { retries: 10 }.should_retry(0));
        assert!(RetryPolicy::MaxRetries { retries: 10 }.should_retry(1));
        assert!(RetryPolicy::MaxRetries { retries: 10 }.should_retry(10));
        assert!(!RetryPolicy::MaxRetries { retries: 10 }.should_retry(11));
        assert!(!RetryPolicy::MaxRetries { retries: 10 }.should_retry(100));
    }

    #[test]
    fn retry_policy_infinite_should_return_when_to_retry() {
        assert!(RetryPolicy::Infinite.should_retry(0));
        assert!(RetryPolicy::Infinite.should_retry(1));
        assert!(RetryPolicy::Infinite.should_retry(10));
        assert!(RetryPolicy::Infinite.should_retry(100));
    }

    #[test]
    fn backoff_policy_none_should_never_wait() {
        assert_eq!(None, BackoffPolicy::None.should_wait(0));
        assert_eq!(None, BackoffPolicy::None.should_wait(1));
        assert_eq!(None, BackoffPolicy::None.should_wait(10));
        assert_eq!(None, BackoffPolicy::None.should_wait(100));
    }

    #[test]
    fn backoff_policy_fixed_should_return_the_wait_time() {
        assert_eq!(None, BackoffPolicy::Fixed { ms: 100 }.should_wait(0));
        assert_eq!(
            Some(Duration::from_millis(100)),
            BackoffPolicy::Fixed { ms: 100 }.should_wait(1)
        );
        assert_eq!(
            Some(Duration::from_millis(100)),
            BackoffPolicy::Fixed { ms: 100 }.should_wait(10)
        );
        assert_eq!(
            Some(Duration::from_millis(1123)),
            BackoffPolicy::Fixed { ms: 1123 }.should_wait(100)
        );
        assert_eq!(None, BackoffPolicy::Fixed { ms: 0 }.should_wait(0));
        assert_eq!(None, BackoffPolicy::Fixed { ms: 0 }.should_wait(1));
        assert_eq!(None, BackoffPolicy::Fixed { ms: 0 }.should_wait(10));
    }

    #[test]
    fn backoff_policy_variable_should_return_the_wait_time() {
        assert_eq!(None, BackoffPolicy::Variable { ms: vec!() }.should_wait(0));
        assert_eq!(None, BackoffPolicy::Variable { ms: vec!() }.should_wait(1));
        assert_eq!(None, BackoffPolicy::Variable { ms: vec!() }.should_wait(200));

        assert_eq!(None, BackoffPolicy::Variable { ms: vec!(0) }.should_wait(0));
        assert_eq!(None, BackoffPolicy::Variable { ms: vec!(0) }.should_wait(1));
        assert_eq!(None, BackoffPolicy::Variable { ms: vec!(0) }.should_wait(100));

        assert_eq!(None, BackoffPolicy::Variable { ms: vec!(100) }.should_wait(0));
        assert_eq!(
            Some(Duration::from_millis(100)),
            BackoffPolicy::Variable { ms: vec!(100) }.should_wait(1)
        );
        assert_eq!(
            Some(Duration::from_millis(100)),
            BackoffPolicy::Variable { ms: vec!(100) }.should_wait(2)
        );
        assert_eq!(
            Some(Duration::from_millis(100)),
            BackoffPolicy::Variable { ms: vec!(100) }.should_wait(10)
        );
        assert_eq!(
            Some(Duration::from_millis(100)),
            BackoffPolicy::Variable { ms: vec!(100) }.should_wait(100)
        );

        assert_eq!(None, BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(0));
        assert_eq!(
            Some(Duration::from_millis(111)),
            BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(1)
        );
        assert_eq!(
            Some(Duration::from_millis(222)),
            BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(2)
        );
        assert_eq!(None, BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(3));
        assert_eq!(
            Some(Duration::from_millis(444)),
            BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(4)
        );
        assert_eq!(
            Some(Duration::from_millis(444)),
            BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(5)
        );
        assert_eq!(
            Some(Duration::from_millis(444)),
            BackoffPolicy::Variable { ms: vec!(111, 222, 0, 444) }.should_wait(100_000)
        );
    }

    #[test]
    fn backoff_policy_exponential_should_return_the_wait_time() {
        assert_eq!(None, BackoffPolicy::Exponential { ms: 123, multiplier: 2 }.should_wait(0));
        assert_eq!(
            Some(Duration::from_millis(123)),
            BackoffPolicy::Exponential { ms: 123, multiplier: 2 }.should_wait(1)
        );
        assert_eq!(
            Some(Duration::from_millis(246)),
            BackoffPolicy::Exponential { ms: 123, multiplier: 2 }.should_wait(2)
        );
        assert_eq!(
            Some(Duration::from_millis(492)),
            BackoffPolicy::Exponential { ms: 123, multiplier: 2 }.should_wait(3)
        );

        assert_eq!(None, BackoffPolicy::Exponential { ms: 1000, multiplier: 3 }.should_wait(0));
        assert_eq!(
            Some(Duration::from_millis(1000)),
            BackoffPolicy::Exponential { ms: 1000, multiplier: 3 }.should_wait(1)
        );
        assert_eq!(
            Some(Duration::from_millis(3000)),
            BackoffPolicy::Exponential { ms: 1000, multiplier: 3 }.should_wait(2)
        );
        assert_eq!(
            Some(Duration::from_millis(9000)),
            BackoffPolicy::Exponential { ms: 1000, multiplier: 3 }.should_wait(3)
        );
    }

    #[test]
    fn retry_policy_should_return_whether_to_retry() {
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: 1 },
            backoff_policy: BackoffPolicy::Fixed { ms: 34 },
        };
        assert_eq!((true, None), retry_strategy.should_retry(0));
        assert_eq!((true, Some(Duration::from_millis(34))), retry_strategy.should_retry(1));
        assert_eq!((false, Some(Duration::from_millis(34))), retry_strategy.should_retry(2));
    }

    #[actix_rt::test]
    async fn should_retry_if_failure() {
        let (sender, mut receiver) = unbounded_channel();
        let attempts = rand::thread_rng().gen_range(10, 250);
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::None,
        };

        let action = Arc::new(Action::new("hello"));

        let executor_addr = RetryActor::start_new(Arc::new(retry_strategy.clone()), move || {
            let executor = AlwaysFailExecutor { sender: sender.clone(), can_retry: true };
            ExecutorRunner::start_new(2, 10, executor).unwrap()
        });

        executor_addr.do_send(ActionMessage { action });

        for _i in 0..=attempts {
            let received = receiver.recv().await.unwrap();
            assert_eq!("hello", received.id);
        }

        actix::clock::delay_for(Duration::from_millis(25)).await;
        // there should be no other messages on the channel
        assert!(receiver.try_recv().is_err());
    }

    #[actix_rt::test]
    async fn should_not_retry_if_ok() {
        let (sender, mut receiver) = unbounded_channel();
        let attempts = rand::thread_rng().gen_range(10, 250);
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::None,
        };

        let action = Arc::new(Action::new("hello"));

        let executor_addr = RetryActor::start_new(Arc::new(retry_strategy.clone()), move || {
            let executor = AlwaysOkExecutor { sender: sender.clone() };
            ExecutorRunner::start_new(2, 10, executor).unwrap()
        });

        executor_addr.do_send(ActionMessage { action });

        let received = receiver.recv().await.unwrap();
        assert_eq!("hello", received.id);

        actix::clock::delay_for(Duration::from_millis(25)).await;
        // there should be no other messages on the channel
        assert!(receiver.try_recv().is_err());
    }

    #[actix_rt::test]
    async fn should_not_retry_if_unrecoverable_error() {
        let (sender, mut receiver) = unbounded_channel();
        let attempts = rand::thread_rng().gen_range(10, 250);
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::None,
        };

        let action = Arc::new(Action::new("hello"));

        let executor_addr = RetryActor::start_new(Arc::new(retry_strategy.clone()), move || {
            let executor = AlwaysFailExecutor { sender: sender.clone(), can_retry: false };
            ExecutorRunner::start_new(2, 10, executor).unwrap()
        });

        executor_addr.do_send(ActionMessage { action });

        let received = receiver.recv().await.unwrap();
        assert_eq!("hello", received.id);

        actix::clock::delay_for(Duration::from_millis(25)).await;
        // there should be no other messages on the channel
        assert!(receiver.try_recv().is_err());
    }

    #[actix_rt::test]
    async fn should_apply_the_backoff_policy_on_failure() {
        let (sender, mut receiver) = unbounded_channel();
        let wait_times = vec![10, 30, 20, 40, 50];
        let attempts = 4;
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::Variable { ms: wait_times.clone() },
        };

        let action = Arc::new(Action::new("hello_world"));

        let executor_addr = RetryActor::start_new(Arc::new(retry_strategy.clone()), move || {
            let executor = AlwaysFailExecutor { sender: sender.clone(), can_retry: true };
            ExecutorRunner::start_new(2, 10, executor).unwrap()
        });

        executor_addr.do_send(ActionMessage { action });

        for i in 0..=(attempts as usize) {
            let before_ms = chrono::Local::now().timestamp_millis();
            let received = receiver.recv().await.unwrap();
            let after_ms = chrono::Local::now().timestamp_millis();

            assert_eq!("hello_world", received.id);
            if i > 0 {
                let passed = after_ms - before_ms;
                let expected = wait_times[i - 1] as i64;
                println!("passed: {} - expected: {}", passed, expected);
                assert!(passed >= expected);
            }
        }

        actix::clock::delay_for(Duration::from_millis(25)).await;
        // there should be no other messages on the channel
        assert!(receiver.try_recv().is_err());
    }

    struct AlwaysFailExecutor {
        can_retry: bool,
        sender: UnboundedSender<Action>,
    }

    impl Executor for AlwaysFailExecutor {
        fn execute(&self, action: &Action) -> Result<(), ExecutorError> {
            self.sender.send(action.clone()).unwrap();
            Err(ExecutorError::ActionExecutionError {
                message: "".to_owned(),
                can_retry: self.can_retry,
                code: None,
            })
        }
    }

    impl std::fmt::Display for AlwaysFailExecutor {
        fn fmt(&self, _fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            Ok(())
        }
    }

    struct AlwaysOkExecutor {
        sender: UnboundedSender<Action>,
    }

    impl Executor for AlwaysOkExecutor {
        fn execute(&self, action: &Action) -> Result<(), ExecutorError> {
            self.sender.send(action.clone()).unwrap();
            Ok(())
        }
    }

    impl std::fmt::Display for AlwaysOkExecutor {
        fn fmt(&self, _fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            Ok(())
        }
    }
}
