//! The `tornado_engine_matcher` crate contains the event processing logic.
//!
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tornado_common_api;

pub mod accessor;
pub mod config;
pub mod error;
pub mod extractor;
pub mod matcher;
pub mod operator;

#[cfg(test)]
extern crate chrono;
