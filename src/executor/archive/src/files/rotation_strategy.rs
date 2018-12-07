use chrono::prelude::*;
use super::FileInfo;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io;
use std::path::Path;

pub enum RotationPolicy {
    Time { pattern: String, current_filename: String },
    Size { size: u64, start_index: u32 },
}

impl RotationPolicy {
    pub fn need_rotate(&self, file: &File, file_info: &FileInfo) -> io::Result<bool> {
        match self {
            RotationPolicy::Size { size, .. } => {
                let metadata = file.metadata()?;
                Ok(metadata.len() > *size)
            },
            RotationPolicy::Time { pattern, current_filename } => {
                let new_name = RotationPolicy::new_filename_with_date_pattern(&pattern, &file_info);
                Ok( !current_filename.eq(&new_name) )
            }
        }
    }

    pub fn new_file(&mut self, file_info: &FileInfo) -> io::Result<File> {
        match *self {
            RotationPolicy::Size { .. } => File::create(&self.get_file_to_open(file_info)?),
            RotationPolicy::Time { ref pattern, ref mut current_filename } => {
                let new_name = RotationPolicy::new_filename_with_date_pattern(pattern, file_info);
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&new_name)
                    .map(|file| {
                        *current_filename = new_name;
                        file
                    })
            }
        }
    }

    fn get_file_to_open(&mut self, file_info: &FileInfo) -> io::Result<String> {
        match self {
            RotationPolicy::Size { start_index, .. } => {
                let mut exists = true;
                let mut filename = "".to_owned();
                create_dir_all(&file_info.dir)?;
                while exists {
                    filename = RotationPolicy::new_filename_with_index(*start_index, file_info);
                    *start_index += 1;
                    let path = Path::new(&filename);
                    exists = path.exists();
                }
                Ok(filename)
            },
            RotationPolicy::Time { .. } => {
                unimplemented!()
            }
        }
    }

    fn new_filename_with_index(index: u32, file_info: &FileInfo) -> String {
        format!(
            "{}/{}-{}.{}",
            file_info.dir, file_info.base_name, index, file_info.extension
        )
    }

    fn new_filename_with_date_pattern(pattern: &str, file_info: &FileInfo) -> String {
        let dt = Local::now();
        let suffix = dt.format(&pattern).to_string();
        format!(
            "{}/{}-{}.{}",
            file_info.dir, file_info.base_name, suffix, file_info.extension
        )
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn size_should_return_false_if_size_less_than_selected() {
        // Arrange
        let file: File = tempfile::tempfile().unwrap();
        let file_info = FileInfo {
            dir: "".to_owned(),
            extension: "".to_owned(),
            base_name: "".to_owned(),
        };
        let rotation_policy = RotationPolicy::Size { size: 5, start_index: 0 };

        // Assert
        assert!(!rotation_policy.need_rotate(&file, &file_info).unwrap());
    }

    #[test]
    fn size_should_return_true_if_size_more_than_selected() {
        // Arrange
        let mut file: File = tempfile::tempfile().unwrap();
        file.write_all(b"hello world!").unwrap();
        let file_info = FileInfo {
            dir: "".to_owned(),
            extension: "".to_owned(),
            base_name: "".to_owned(),
        };
        let rotation_policy = RotationPolicy::Size { size: 5, start_index: 0 };

        // Assert
        assert!(rotation_policy.need_rotate(&file, &file_info).unwrap());
    }

    #[test]
    fn size_should_return_new_filename() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut rotation_policy = RotationPolicy::Size { size: 5, start_index: 0 };
        let file_info = FileInfo {
            dir: dir.to_owned(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };

        {
            assert_eq!(format!("{}/base-0.ext", dir), rotation_policy.get_file_to_open(&file_info).unwrap());
            match rotation_policy {
                RotationPolicy::Size { size: _, start_index } => {
                    assert_eq!(1, start_index);
                },
                _ => assert!(false)
            }
        }

        {
            assert_eq!(format!("{}/base-1.ext", dir), rotation_policy.get_file_to_open(&file_info).unwrap());
            match rotation_policy {
                RotationPolicy::Size { size: _, start_index } => {
                    assert_eq!(2, start_index);
                },
                _ => assert!(false)
            }
        }
    }

    #[test]
    fn size_should_create_a_new_file() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let mut rotation_policy = RotationPolicy::Size { size: 5, start_index: 0 };
        let file_info = FileInfo {
            dir: tempdir.path().to_str().unwrap().to_owned(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };
        let expected_path =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 0, file_info.extension);

        // Act
        let mut file = rotation_policy.new_file(&file_info).unwrap();
        file.write_all(b"hello world 1").unwrap();
        let file_content = fs::read_to_string(expected_path).unwrap();

        // Assert
        assert_eq!("hello world 1", &file_content);
    }

    #[test]
    fn size_should_skip_existing_files_and_create_a_new_one() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let mut rotation_policy = RotationPolicy::Size { size: 5, start_index: 0 };
        let file_info = FileInfo {
            dir: tempdir.path().to_str().unwrap().to_owned(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };
        let expected_path_0 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 0, file_info.extension);
        fs::write(&expected_path_0, "hello world 1").unwrap();

        let expected_path_1 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 1, file_info.extension);
        fs::write(&expected_path_1, "hello world 12").unwrap();

        let expected_path_2 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 2, file_info.extension);
        let expected_path_3 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 3, file_info.extension);

        // Act
        let mut file = rotation_policy.new_file(&file_info).unwrap();
        file.write_all(b"hello world 123").unwrap();
        // should rotate to new file
        file = rotation_policy.new_file(&file_info).unwrap();
        file.write_all(b"hello world 1234").unwrap();

        // Assert
        assert_eq!("hello world 1", &fs::read_to_string(&expected_path_0).unwrap());
        assert_eq!("hello world 12", &fs::read_to_string(&expected_path_1).unwrap());
        assert_eq!("hello world 123", &fs::read_to_string(&expected_path_2).unwrap());
        assert_eq!("hello world 1234", &fs::read_to_string(&expected_path_3).unwrap());
    }


    #[test]
    fn time_should_return_rotate_true_if_pattern_does_not_match() {
        // Arrange
        let file: File = tempfile::tempfile().unwrap();

        let file_info = FileInfo {
            dir: "/dir".to_owned(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };

        let pattern = "%Y";
        let rotation_policy = RotationPolicy::Time { pattern: pattern.to_owned(), current_filename: "/dir/base-1900.ext".to_owned() };

        // Assert
        assert!(rotation_policy.need_rotate(&file, &file_info).unwrap());
    }

    #[test]
    fn time_should_return_rotate_false_if_pattern_matches_current() {
        // Arrange
        let file: File = tempfile::tempfile().unwrap();

        let file_info = FileInfo {
            dir: "/dir".to_owned(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };

        let pattern = "%Y";
        let current_year = Local::now().format(&pattern).to_string();
        let rotation_policy = RotationPolicy::Time { pattern: pattern.to_owned(), current_filename: format!("/dir/base-{}.ext", &current_year) };

        // Assert
        assert!(!rotation_policy.need_rotate(&file, &file_info).unwrap());
    }

    #[test]
    fn time_should_return_new_file() {

        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        let file_info = FileInfo {
            dir: dir.clone(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };

        let pattern = "%Y";
        let mut rotation_policy = RotationPolicy::Time { pattern: pattern.to_owned(), current_filename: "".to_owned() };

        let file_path = RotationPolicy::new_filename_with_date_pattern(&pattern, &file_info);

        // Act
        let mut file = rotation_policy.new_file(&file_info).unwrap();
        file.write_all(b"hello world!").unwrap();
        file.flush().unwrap();

        // Assert
        assert_eq!("hello world!", &fs::read_to_string(&file_path).unwrap());
    }

    #[test]
    fn time_should_append_to_existing_file() {

        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        let file_info = FileInfo {
            dir: dir.clone(),
            extension: "ext".to_owned(),
            base_name: "base".to_owned(),
        };

        let pattern = "%Y";
        let mut rotation_policy = RotationPolicy::Time { pattern: pattern.to_owned(), current_filename: "".to_owned() };

        let file_path = RotationPolicy::new_filename_with_date_pattern(&pattern, &file_info);

        fs::write(&file_path, "existing_data-").unwrap();

        // Act
        let mut file = rotation_policy.new_file(&file_info).unwrap();
        file.write_all(b"hello world!").unwrap();
        file.flush().unwrap();

        // Assert
        assert_eq!("existing_data-hello world!", &fs::read_to_string(&file_path).unwrap());
    }
}
