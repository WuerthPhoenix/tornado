extern crate chrono;
#[macro_use]
extern crate log;
extern crate serde;
extern crate serde_json;
extern crate tornado_common_api;
extern crate tornado_executor_common;

use std::io::prelude::*;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

pub mod files;

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
mod test {

    use super::*;
    use files::rotation_strategy::RotationPolicy;
    use files::{FileInfo, RotateFileWriter};
    use std::fs;
    use std::io::{BufRead, BufReader};
    use tornado_common_api::Event;

    #[test]
    fn should_write_one_event_per_line() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let rotation_policy = RotationPolicy::Size { size: 10000, start_index: 0 };
        let file_info = FileInfo {
            dir: tempdir.path().to_str().unwrap().to_owned(),
            extension: "log".to_owned(),
            base_name: "base".to_owned(),
        };

        let expected_path =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 0, file_info.extension);

        let writer = RotateFileWriter::new(file_info, rotation_policy).unwrap();
        let mut archiver = ArchiveExecutor::new(writer);

        let attempts = 10;
        let mut sent_events = vec![];
        let mut read_lines = vec![];

        // Act
        for i in 0..attempts {
            let event = Event::new(format!("event-{}", i));
            sent_events.push(event.clone());
            let mut action = Action::new(format!("action-{}", i));
            action.payload.insert("event".to_owned(), event.into());
            archiver.execute(&action).unwrap()
        }

        let file = fs::File::open(&expected_path).unwrap();
        for line in BufReader::new(file).lines() {
            let line_string = line.unwrap();
            println!("Read line: {}", &line_string);
            read_lines.push(line_string);
        }

        // Assert
        assert_eq!(attempts, sent_events.len());
        assert_eq!(attempts, read_lines.len());

        for i in 0..attempts {
            let event_from_file =
                serde_json::from_str::<Event>(read_lines.get(i).unwrap()).unwrap();
            assert_eq!(sent_events.get(i).unwrap(), &event_from_file)
        }
    }

    #[test]
    fn should_never_split_a_single_event_in_two_different_files() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let rotation_policy = RotationPolicy::Size { size: 1234, start_index: 0 };
        let file_info = FileInfo {
            dir: dir.clone(),
            extension: "log".to_owned(),
            base_name: "base".to_owned(),
        };

        let writer = RotateFileWriter::with_capacity(100, file_info, rotation_policy).unwrap();
        let mut archiver = ArchiveExecutor::new(writer);

        let attempts = 10001;

        // Act
        for i in 0..attempts {
            let event =
                Event::new(format!("event-{}{}{}{}{}{}{}{}{}{}", i, i, i, i, i, i, i, i, i, i));
            let mut action = Action::new(format!("action-{}", i));
            action.payload.insert("event".to_owned(), event.into());
            archiver.execute(&action).unwrap()
        }

        // Assert
        let mut found_files = 0;
        let mut found = true;

        while found {
            let file_path = format!("{}/base-{}.log", &dir, found_files);

            match fs::File::open(&file_path) {
                Ok(file) => {
                    found_files += 1;
                    for line in BufReader::new(file).lines() {
                        let line_string = line.unwrap();
                        println!("Read line: {}", &line_string);
                        if !line_string.is_empty() {
                            serde_json::from_str::<Event>(&line_string).unwrap();
                        }
                    }
                }
                Err(_) => found = false,
            }
        }

        println!("found {} files", &found_files);
        assert!(found_files > 0);
    }

}
