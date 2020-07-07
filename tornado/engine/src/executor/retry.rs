use serde::{Deserialize, Serialize};
use std::time::Duration;
use tornado_executor_common::{Executor, ExecutorError};
use crate::executor::{ExecutorActor, ActionMessage};
use actix::{Actor, Context, Addr, SyncArbiter, Handler};
use log::*;
use std::sync::Arc;

/// Defines the strategy to apply in case of a failure.
/// This is applied, for example, when an action execution fails
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RetryStrategy {
    pub retry_policy: RetryPolicy,
    pub backoff_policy: BackoffPolicy,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self { retry_policy: RetryPolicy::None, backoff_policy: BackoffPolicy::None }
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
    MaxAttempts { attempts: u32 },
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
                RetryPolicy::MaxAttempts { attempts } => *attempts + 1 > failed_attempts,
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
    /// Permits to specify the amount of time between two following retry attempts.
    /// The time to wait after 'i' retries is specified in the vector at position 'i'.
    /// If the number of retries is bigger than the vector length, then the last value in the vector is used.
    /// For example:
    /// ms = [111,222,333] -> It waits 111 ms after the first failure, 222 ms after the second failure and then 333 ms for all following failures.
    Variable { ms: Vec<u32> },
    // Exponential
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
            }
        }
    }
}

pub struct RetryActor<E: Executor + std::fmt::Display + Unpin + 'static> {
    executor_addr: Addr<ExecutorActor<E>>,
    retry_strategy: Arc<RetryStrategy>,
}

impl <E: Executor + std::fmt::Display + Unpin + 'static> Actor for RetryActor<E> {
    type Context = Context<Self>;
}

impl <E: Executor + std::fmt::Display + Unpin> RetryActor<E> {
    pub fn start_new<F>(threads: usize, retry_strategy: Arc<RetryStrategy>, factory: F) -> Addr<Self>
        where
            F: Fn() -> ExecutorActor<E> + Send + Sync + 'static {

        let executor_addr = SyncArbiter::start(threads, move || {
            factory()
        });

        Self {
            retry_strategy,
            executor_addr,
        }.start()
    }
}

impl <E: Executor + std::fmt::Display + Unpin + 'static> Handler<ActionMessage> for RetryActor<E> {
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, mut msg: ActionMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("RetryActor - received new message");
        let executor_addr = self.executor_addr.clone();
        let retry_strategy = self.retry_strategy.clone();
        actix::spawn(async move {
            let mut should_retry = true;
            while should_retry {
                should_retry = false;
                let result = executor_addr.send(msg.clone()).await;
                match result {
                    Ok(response) => {
                        if let Err(_) = response {
                            msg.failed_attempts = msg.failed_attempts + 1;
                            let (new_should_retry, should_wait) = retry_strategy.should_retry(msg.failed_attempts);
                            should_retry = new_should_retry;
                            if should_retry {
                                debug!("The failed message will be reprocessed based on the current RetryPolicy. Message: {:?}", msg);
                                if let Some(delay_for) = should_wait {
                                    actix::clock::delay_for(delay_for).await;
                                }
                            } else {
                                warn!("The failed message will not be retried any more in respect of the current RetryPolicy. Message: {:?}", msg)
                            }
                        }
                    },
                    Err(e) => error!("MailboxError: {}", e)
                }
            }
        });
        Ok(())
    }
}


#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    fn retry_policy_should_return_when_to_retry() {
        // None
        assert!(RetryPolicy::None.should_retry(0));
        assert!(!RetryPolicy::None.should_retry(1));
        assert!(!RetryPolicy::None.should_retry(10));
        assert!(!RetryPolicy::None.should_retry(100));

        // Max
        assert!(RetryPolicy::MaxAttempts { attempts: 0 }.should_retry(0));
        assert!(!RetryPolicy::MaxAttempts { attempts: 0 }.should_retry(1));
        assert!(!RetryPolicy::MaxAttempts { attempts: 0 }.should_retry(10));
        assert!(!RetryPolicy::MaxAttempts { attempts: 0 }.should_retry(100));

        assert!(RetryPolicy::MaxAttempts { attempts: 1 }.should_retry(0));
        assert!(RetryPolicy::MaxAttempts { attempts: 1 }.should_retry(1));
        assert!(!RetryPolicy::MaxAttempts { attempts: 1 }.should_retry(2));
        assert!(!RetryPolicy::MaxAttempts { attempts: 1 }.should_retry(10));
        assert!(!RetryPolicy::MaxAttempts { attempts: 1 }.should_retry(100));

        assert!(RetryPolicy::MaxAttempts { attempts: 10 }.should_retry(0));
        assert!(RetryPolicy::MaxAttempts { attempts: 10 }.should_retry(1));
        assert!(RetryPolicy::MaxAttempts { attempts: 10 }.should_retry(10));
        assert!(!RetryPolicy::MaxAttempts { attempts: 10 }.should_retry(11));
        assert!(!RetryPolicy::MaxAttempts { attempts: 10 }.should_retry(100));

        // Infinite
        assert!(RetryPolicy::Infinite.should_retry(0));
        assert!(RetryPolicy::Infinite.should_retry(1));
        assert!(RetryPolicy::Infinite.should_retry(10));
        assert!(RetryPolicy::Infinite.should_retry(100));
    }

    #[test]
    fn backoff_policy_should_return_the_wait_time() {
        // None
        assert_eq!(None, BackoffPolicy::None.should_wait(0));
        assert_eq!(None, BackoffPolicy::None.should_wait(1));
        assert_eq!(None, BackoffPolicy::None.should_wait(10));
        assert_eq!(None, BackoffPolicy::None.should_wait(100));

        // Fixed
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

        // Variable
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
    fn retry_policy_should_return_whether_to_retry() {
        let retry_strategy = RetryStrategy {
            retry_policy: RetryPolicy::MaxAttempts { attempts: 1 },
            backoff_policy: BackoffPolicy::Fixed { ms: 34 },
        };
        assert_eq!((true, None), retry_strategy.should_retry(0));
        assert_eq!((true, Some(Duration::from_millis(34))), retry_strategy.should_retry(1));
        assert_eq!((false, Some(Duration::from_millis(34))), retry_strategy.should_retry(2));
    }
}
