//! The `tornado_engine_matcher` crate contains the event processing logic.
//!
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate regex;
extern crate serde;
extern crate tornado_common_api;
#[macro_use]
extern crate serde_derive;

pub mod accessor;
pub mod config;
pub mod error;
pub mod operator;

#[cfg(test)]
extern crate chrono;
