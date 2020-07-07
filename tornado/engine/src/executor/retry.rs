use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Defines the strategy to apply in case of a failure.
/// This is applied, for example, when an action execution fails
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RetryStrategy {
    pub retry_policy: RetryPolicy,
    pub backoff_policy: BackoffPolicy,
}

impl RetryStrategy {

    /// Returns whether a retry attempt should be performed and an optional backoff time
    pub fn should_retry(&self, failed_attempts: u32) -> (bool, Option<Duration>) {
        (self.retry_policy.should_retry(failed_attempts), self.backoff_policy.should_wait(failed_attempts))
    }

}

// Defines the retry policy of a RetryStrategy
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum RetryPolicy {
    /// No Retry attempts defined
    None,
    /// The operation will be retried for a max number of times.
    MaxAttempts{max: u32},
    /// The operation will be retried an infinite number of times.
    Infinite,
    // Timeout,
}

impl RetryPolicy {

    pub fn should_retry(&self, failed_attempts: u32) -> bool {
        if failed_attempts == 0 {
            true
        } else {
            match self {
                RetryPolicy::None => false,
                RetryPolicy::Infinite => true,
                RetryPolicy::MaxAttempts { max} => {
                    *max+1 > failed_attempts
                }
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
    /// Fixed amount ot time between each retry attempt will be waited
    Fixed{ms: u32},
    /// Permits to specify the amount of time between two following retry attempts.
    /// The time to wait after 'i' retries is specified in the vector at position 'i'.
    /// If the number of retries is bigger than the vector lenght, then the last value in the vector is used.
    Variable{ms: Vec<u32>},
    // Exponential
}

impl BackoffPolicy {

    pub fn should_wait(&self, failed_attempts: u32) -> Option<Duration> {
        if failed_attempts == 0 {
            None
        } else {
            match self {
                BackoffPolicy::None => None,
                BackoffPolicy::Fixed{ms} => if *ms > 0 {
                    Some(Duration::from_millis(*ms as u64))
                } else {
                    None
                },
                BackoffPolicy::Variable{ms} => {
                    let index = (failed_attempts - 1) as usize;
                    let option_wait_ms = if ms.len() > index {
                        ms.get(index)
                    } else {
                        ms.last()
                    };
                    match option_wait_ms {
                        Some(wait_ms) => if *wait_ms > 0 {
                            Some(Duration::from_millis(*wait_ms as u64))
                        } else {
                            None
                        },
                        None => None
                    }
                }
            }
        }
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
        assert!(RetryPolicy::MaxAttempts {max: 0}.should_retry(0));
        assert!(!RetryPolicy::MaxAttempts {max: 0}.should_retry(1));
        assert!(!RetryPolicy::MaxAttempts {max: 0}.should_retry(10));
        assert!(!RetryPolicy::MaxAttempts {max: 0}.should_retry(100));

        assert!(RetryPolicy::MaxAttempts {max: 1}.should_retry(0));
        assert!(RetryPolicy::MaxAttempts {max: 1}.should_retry(1));
        assert!(!RetryPolicy::MaxAttempts {max: 1}.should_retry(2));
        assert!(!RetryPolicy::MaxAttempts {max: 1}.should_retry(10));
        assert!(!RetryPolicy::MaxAttempts {max: 1}.should_retry(100));

        assert!(RetryPolicy::MaxAttempts {max: 10}.should_retry(0));
        assert!(RetryPolicy::MaxAttempts {max: 10}.should_retry(1));
        assert!(RetryPolicy::MaxAttempts {max: 10}.should_retry(10));
        assert!(!RetryPolicy::MaxAttempts {max: 10}.should_retry(11));
        assert!(!RetryPolicy::MaxAttempts {max: 10}.should_retry(100));

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
        assert_eq!(None, BackoffPolicy::Fixed {ms: 100}.should_wait(0));
        assert_eq!(Some(Duration::from_millis(100)), BackoffPolicy::Fixed {ms: 100}.should_wait(1));
        assert_eq!(Some(Duration::from_millis(100)), BackoffPolicy::Fixed {ms: 100}.should_wait(10));
        assert_eq!(Some(Duration::from_millis(1123)), BackoffPolicy::Fixed {ms: 1123}.should_wait(100));

        assert_eq!(None, BackoffPolicy::Fixed {ms: 0}.should_wait(0));
        assert_eq!(None, BackoffPolicy::Fixed {ms: 0}.should_wait(1));
        assert_eq!(None, BackoffPolicy::Fixed {ms: 0}.should_wait(10));

        // Variable
        assert_eq!(None, BackoffPolicy::Variable {ms: vec!()}.should_wait(0));
        assert_eq!(None, BackoffPolicy::Variable {ms: vec!()}.should_wait(1));
        assert_eq!(None, BackoffPolicy::Variable {ms: vec!()}.should_wait(200));

        assert_eq!(None, BackoffPolicy::Variable {ms: vec!(0)}.should_wait(0));
        assert_eq!(None, BackoffPolicy::Variable {ms: vec!(0)}.should_wait(1));
        assert_eq!(None, BackoffPolicy::Variable {ms: vec!(0)}.should_wait(100));

        assert_eq!(None, BackoffPolicy::Variable {ms: vec!(100)}.should_wait(0));
        assert_eq!(Some(Duration::from_millis(100)), BackoffPolicy::Variable {ms: vec!(100)}.should_wait(1));
        assert_eq!(Some(Duration::from_millis(100)), BackoffPolicy::Variable {ms: vec!(100)}.should_wait(2));
        assert_eq!(Some(Duration::from_millis(100)), BackoffPolicy::Variable {ms: vec!(100)}.should_wait(10));
        assert_eq!(Some(Duration::from_millis(100)), BackoffPolicy::Variable {ms: vec!(100)}.should_wait(100));

        assert_eq!(None, BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(0));
        assert_eq!(Some(Duration::from_millis(111)), BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(1));
        assert_eq!(Some(Duration::from_millis(222)), BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(2));
        assert_eq!(None, BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(3));
        assert_eq!(Some(Duration::from_millis(444)), BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(4));
        assert_eq!(Some(Duration::from_millis(444)), BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(5));
        assert_eq!(Some(Duration::from_millis(444)), BackoffPolicy::Variable {ms: vec!(111,222,0,444)}.should_wait(100_000));
    }

    #[test]
    fn retry_policy_should_return_whether_to_retry() {

    }
}