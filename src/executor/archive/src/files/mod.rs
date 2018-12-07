pub mod rotation_strategy;

use std::fs::File;
use std::io;
use std::io::prelude::*;

/// The information required to create or open a file from a RotateFileWriter.
pub struct FileInfo {
    /// The directory where the file(s) should be written in.
    pub dir: String,
    /// The file base name. The final file name is determined by the RotationPolicy.
    /// For example, it could have a progressive numerical suffix or a suffix specifying the creation date.
    pub base_name: String,
    /// The file extension.
    pub extension: String,
}

/// A RotateFileWriter is a std::io::Writer that automatically rotates the underlying file
/// when the conditions specified by the RotationPolicy are met.
pub struct RotateFileWriter {
    buffer_size: usize,
    file_info: FileInfo,
    rotation_policy: rotation_strategy::RotationPolicy,
    writer: io::BufWriter<File>,
}

impl RotateFileWriter {
    pub fn new(
        file_info: FileInfo,
        rotation_policy: rotation_strategy::RotationPolicy,
    ) -> io::Result<RotateFileWriter> {
        RotateFileWriter::with_capacity(4096, file_info, rotation_policy)
    }

    pub fn with_capacity(
        buffer_size: usize,
        file_info: FileInfo,
        mut rotation_policy: rotation_strategy::RotationPolicy,
    ) -> io::Result<RotateFileWriter> {
        let writer = RotateFileWriter::new_writer(buffer_size, &file_info, &mut rotation_policy)?;
        Ok(RotateFileWriter { buffer_size, file_info, rotation_policy, writer })
    }

    fn check_need_rotate(&mut self) -> io::Result<()> {
        if self.rotation_policy.need_rotate(self.writer.get_ref(), &self.file_info)? {
            self.writer = RotateFileWriter::new_writer(
                self.buffer_size,
                &self.file_info,
                &mut self.rotation_policy,
            )?;
        };
        Ok(())
    }

    fn new_writer(
        capacity: usize,
        file_info: &FileInfo,
        rotation_policy: &mut rotation_strategy::RotationPolicy,
    ) -> io::Result<io::BufWriter<File>> {
        Ok(io::BufWriter::with_capacity(capacity, rotation_policy.new_file(file_info)?))
    }
}

impl Write for RotateFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.check_need_rotate()?;
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.check_need_rotate()?;
        self.writer.write_all(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn should_rotate_the_file_when_limit_is_reached() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let rotation_policy = rotation_strategy::RotationPolicy::Size { size: 5, start_index: 0 };
        let file_info = FileInfo {
            dir: tempdir.path().to_str().unwrap().to_owned(),
            extension: "log".to_owned(),
            base_name: "base".to_owned(),
        };

        let expected_path_0 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 0, file_info.extension);
        let expected_path_1 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 1, file_info.extension);
        let expected_path_2 =
            format!("{}/{}-{}.{}", file_info.dir, file_info.base_name, 2, file_info.extension);

        let mut writer = RotateFileWriter::new(file_info, rotation_policy).unwrap();

        // Act
        for _i in 0..6 {
            writer.write_all(b"1234").unwrap();
            writer.flush().unwrap();
        }

        // Assert
        assert_eq!("12341234", &fs::read_to_string(&expected_path_0).unwrap());
        assert_eq!("12341234", &fs::read_to_string(&expected_path_1).unwrap());
        assert_eq!("12341234", &fs::read_to_string(&expected_path_2).unwrap());
    }

}
