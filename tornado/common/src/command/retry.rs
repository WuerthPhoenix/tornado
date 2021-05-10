use crate::command::Command;
use core::fmt::Debug;
use core::marker::PhantomData;
use log::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tornado_common_api::RetriableError;

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

/// A Command that reties a failing operation based on the specified RetryStrategy
pub struct RetryCommand<I: Clone + Debug, O, E: RetriableError, T: Command<I, Result<O, E>>> {
    command: T,
    retry_strategy: RetryStrategy,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
    phantom_e: PhantomData<E>,
}

impl<I: Clone + Debug, O, E: RetriableError, T: Command<I, Result<O, E>>> RetryCommand<I, O, E, T> {
    pub fn new(retry_strategy: RetryStrategy, command: T) -> Self {
        Self {
            retry_strategy,
            command,
            phantom_i: PhantomData,
            phantom_o: PhantomData,
            phantom_e: PhantomData,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<I: Clone + Debug, O, E: RetriableError, T: Command<I, Result<O, E>>> Command<I, Result<O, E>>
    for RetryCommand<I, O, E, T>
{
    async fn execute(&self, message: I) -> Result<O, E> {
        trace!("RetryCommand - received new message");

        let command = &self.command;
        let retry_strategy = &self.retry_strategy;

        let mut should_retry = true;
        let mut failed_attempts = 0;
        while should_retry {
            let result = command.execute(message.clone()).await;
            match result {
                Ok(response) => {
                    return Ok(response);
                }
                Err(err) => {
                    if !err.can_retry() {
                        warn!("The failed message will not be retried as the error is not recoverable.");
                        return Err(err);
                    } else {
                        failed_attempts += 1;
                        let (new_should_retry, should_wait) =
                            retry_strategy.should_retry(failed_attempts);
                        should_retry = new_should_retry;

                        if should_retry {
                            debug!("The failed message will be reprocessed based on the current RetryPolicy. Failed attempts: {}. Message: {:?}", failed_attempts, message);
                            if let Some(delay_for) = should_wait {
                                debug!("Wait for {:?} before retrying.", delay_for);
                                actix::clock::delay_for(delay_for).await;
                            }
                        } else {
                            warn!("The failed message will not be retried any more in respect of the current RetryPolicy. Failed attempts: {}. Message: {:?}", failed_attempts, message);
                            return Err(err);
                        }
                    }
                }
            }
        }
        unreachable!();
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use rand::Rng;
    use std::rc::Rc;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
    use tornado_common_api::Action;
    use tornado_executor_common::{ExecutorError, StatelessExecutor};

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
        let attempts = rand::thread_rng().gen_range(10..250);
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::None,
        };

        let action = Rc::new(Action::new("hello"));

        let command = RetryCommand::new(
            retry_strategy.clone(),
            AlwaysFailExecutor { sender: sender.clone(), can_retry: true },
        );

        actix::spawn(async move {
            let _res = command.execute(action).await;
        });

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
        let attempts = rand::thread_rng().gen_range(10..250);
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::None,
        };

        let action = Rc::new(Action::new("hello"));

        let command =
            RetryCommand::new(retry_strategy.clone(), AlwaysOkExecutor { sender: sender.clone() });

        actix::spawn(async move {
            let _res = command.execute(action).await;
        });

        let received = receiver.recv().await.unwrap();
        assert_eq!("hello", received.id);

        actix::clock::delay_for(Duration::from_millis(25)).await;
        // there should be no other messages on the channel
        assert!(receiver.try_recv().is_err());
    }

    #[actix_rt::test]
    async fn should_not_retry_if_unrecoverable_error() {
        let (sender, mut receiver) = unbounded_channel();
        let attempts = rand::thread_rng().gen_range(10..250);
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxRetries { retries: attempts },
            backoff_policy: BackoffPolicy::None,
        };

        let action = Rc::new(Action::new("hello"));

        let command = RetryCommand::new(
            retry_strategy.clone(),
            AlwaysFailExecutor { sender: sender.clone(), can_retry: false },
        );

        actix::spawn(async move {
            let _res = command.execute(action).await;
        });

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

        let action = Rc::new(Action::new("hello_world"));

        let command = RetryCommand::new(
            retry_strategy.clone(),
            AlwaysFailExecutor { sender: sender.clone(), can_retry: true },
        );

        actix::spawn(async move {
            let _res = command.execute(action).await;
        });

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
        sender: UnboundedSender<Rc<Action>>,
    }

    #[async_trait::async_trait(?Send)]
    impl StatelessExecutor for AlwaysFailExecutor {
        async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
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
        sender: UnboundedSender<Rc<Action>>,
    }

    #[async_trait::async_trait(?Send)]
    impl StatelessExecutor for AlwaysOkExecutor {
        async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
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
