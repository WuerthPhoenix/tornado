//! The `tornado_engine_matcher` crate contains the event processing logic.
//!


extern crate chrono;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate regex;
extern crate tornado_common;

pub mod extractor;
pub mod rule;
