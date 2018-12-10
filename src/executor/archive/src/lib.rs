#[macro_use]
extern crate log;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tornado_common_api;
extern crate tornado_executor_common;

use std::io::prelude::*;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

mod groups;

pub struct ArchiveExecutor<W: Write> {
    writer: W,
}

impl<W: Write> ArchiveExecutor<W> {
    pub fn new(writer: W) -> ArchiveExecutor<W> {
        ArchiveExecutor { writer }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), ExecutorError> {
        self.writer.write_all(buf).map_err(|err| ExecutorError::ActionExecutionError {
            message: format!("Cannot write to file: {}", err),
        })
    }

    fn flush(&mut self) -> Result<(), ExecutorError> {
        self.writer.flush().map_err(|err| ExecutorError::ActionExecutionError {
            message: format!("Cannot flush to file: {}", err),
        })
    }
}

impl<W: Write> Executor for ArchiveExecutor<W> {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        debug!("ArchiveExecutor - received action: \n{:#?}", action);

        let event =
            action.payload.get("event").ok_or_else(|| ExecutorError::ActionExecutionError {
                message: "Expected the event to be in action payload.".to_owned(),
            })?;
        let event_bytes = serde_json::to_vec(event).unwrap();

        self.write_all(&event_bytes)?;
        self.write_all(b"\n")?;
        self.flush()?;

        Ok(())
    }
}

#[cfg(test)]
extern crate tempfile;

#[cfg(test)]
mod test {}
