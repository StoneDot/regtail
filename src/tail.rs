/*
 * Copyright 2019 StoneDot (Hiroaki Goto)
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::cmp::max;
use std::fs::File;
use std::io::{self, Read, Result, Seek, SeekFrom, Stdout, Write};
use std::path::PathBuf;

// Max recommended buffer size is 128kB
// We choose reasonable size 8kB
const BUFFER_SIZE: usize = 8 * 1024;
const BUFFER_LEN: u64 = BUFFER_SIZE as u64;

pub trait Length {
    fn len(self: &Self) -> Result<u64>;
}

impl Length for std::fs::File {
    fn len(self: &Self) -> Result<u64> {
        Ok(self.metadata()?.len())
    }
}

pub struct TailState<T, U> where
    T: Read + Seek + Length,
    U: Write {
    reader: T,
    writer: U,
    reader_seek_pos: u64,
    printed_eol: bool,
}

impl TailState<std::fs::File, io::BufWriter<Stdout>> {
    pub fn from_file(mut file: File) -> Result<TailState<File, io::BufWriter<Stdout>>> {
        let pos = file.seek(SeekFrom::Current(0))?;
        let writer = io::BufWriter::new(io::stdout());
        Ok(TailState {
            reader: file,
            writer,
            reader_seek_pos: pos,
            printed_eol: false,
        })
    }
}

impl<T, U> TailState<T, U> where
    T: Read + Seek + Length,
    U: Write {
    pub fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        self.reader.read(&mut buf)
    }

    pub fn write(&mut self, mut buf: &[u8]) -> Result<usize> {
        self.writer.write(&mut buf)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }

    pub fn seek(mut self: &mut Self, seek: SeekFrom) -> Result<u64> {
        self.reader_seek_pos = self.reader.seek(seek)?;
        Ok(self.reader_seek_pos)
    }

    pub fn current_seek(self: &Self) -> u64 {
        self.reader_seek_pos
    }

    pub fn len(self: &Self) -> Result<u64> {
        self.reader.len()
    }

    pub fn printed_eol(self: &Self) -> bool { self.printed_eol }

    fn tail_start_position(self: &mut Self, tail_count: u64) -> Result<u64> {
        let mut buffer = [0u8; BUFFER_SIZE];

        // Read file from tail requires file size
        let len = self.len()?;

        // Empty file consideration
        if len == 0 {
            return Ok(0);
        }

        // Skip EOS
        let end_index = len - 1;
        if end_index <= 0 {
            return Ok(0);
        }

        // Seek position should be a multiple of BUFFER_SIZE because of read efficiency
        let mut size = end_index % BUFFER_LEN;
        if size == 0 {
            size = BUFFER_LEN;
        }
        let mut start_index = max(0, end_index - size);
        assert_eq!(0, start_index % BUFFER_LEN);

        // Read to buffer
        self.seek(SeekFrom::Start(start_index))?;
        let mut read_size = self.read(&mut buffer)?;

        let mut target = &buffer[..read_size];

        // Skip last line ending
        if let Some(&x) = target.last() {
            if x == b'\n' {
                target = &target[..read_size - 1];
            }
        }

        let mut eol_count = 0;
        loop {
            // Count end of lines
            for (i, &byte) in target.iter().enumerate().rev() {
                if byte == b'\n' {
                    eol_count += 1;
                    if eol_count >= tail_count {
                        return Ok(start_index + i as u64 + 1);
                    }
                }
            }

            // End check
            if start_index == 0 {
                return Ok(0);
            }

            // Read file data into buffer
            start_index = start_index - BUFFER_LEN;
            self.seek(SeekFrom::Start(start_index))?;
            read_size = self.read(&mut buffer)?;
            target = &buffer[..read_size];
        }
    }

    pub fn handle_shrink(self: &mut Self, offset: u64) -> Result<bool> {
        let len = self.len()?;
        if len < offset {
            self.seek(SeekFrom::Start(0))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn seek_with_shrink_handling(self: &mut Self, offset: u64) -> Result<u64> {
        // Shrink handling
        if self.handle_shrink(offset)? {
            return Ok(0);
        }

        // Seek to target position
        self.seek(SeekFrom::Start(offset))
    }

    pub fn dump_to_tail(self: &mut Self) -> Result<u64> {
        let mut buffer = [0; BUFFER_SIZE];
        let mut offset = self.current_seek();
        let initial_size = (BUFFER_LEN - (offset % BUFFER_LEN)) as usize;
        let mut target = &mut buffer[..initial_size];

        // Read initial data
        let read_size = self.read(&mut target)?;
        target = &mut target[..read_size];
        offset += read_size as u64;

        // Hold the byte last read
        let mut last_byte = target.last().map(u8::to_owned);

        if read_size == 0 {
            return Ok(offset);
        } else {
            loop {
                // Write to stdio
                self.write(&target)?;

                // Read additional data
                let read_size = self.read(&mut buffer)?;
                target = &mut buffer[..read_size];
                offset += read_size as u64;
                if read_size == 0 {
                    // Flush buffer
                    self.flush()?;

                    // Save whether last byte is \n
                    self.printed_eol = last_byte.map_or(false, |byte| byte == b'\n');

                    return Ok(offset);
                }

                last_byte = target.last().map(u8::to_owned);
            }
        }
    }
}

fn tail_from_reader<T, U>(reader: &mut TailState<T, U>, tail_count: u64)
                          -> Result<u64> where
    T: Read + Seek + Length,
    U: Write {
    let offset = reader.tail_start_position(tail_count)?;
    reader.seek_with_shrink_handling(offset)?;
    reader.dump_to_tail()
}

pub fn tail(path: &PathBuf, tail_count: u64) -> Result<TailState<File, io::BufWriter<Stdout>>> {
    let file = File::open(path)?;
    let mut reader = TailState::from_file(file)?;
    let _offset = tail_from_reader(&mut reader, tail_count)?;
    Ok(reader)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::io::Result;

    use super::Length;
    use super::tail_from_reader;
    use super::TailState;

    const CONTENT: &str = r#"line1
line2
line3
line4
line5
"#;

    const CONTENT_WITHOUT_LINE_ENDING: &str = r#"line1
line2
line3
line4
line5"#;

    impl Length for Cursor<&[u8]> {
        fn len(self: &Self) -> Result<u64> {
            Ok(self.get_ref().len() as u64)
        }
    }

    impl TailState<Cursor<&[u8]>, &mut Vec<u8>> {
        pub fn from_slice<'a>(reader: Cursor<&'a [u8]>, writer: &'a mut Vec<u8>)
                              -> Result<TailState<Cursor<&'a [u8]>, &'a mut Vec<u8>>> {
            Ok(TailState {
                reader,
                writer,
                reader_seek_pos: 0,
                printed_eol: false,
            })
        }
    }

    macro_rules! tail_state_test {
        ( $variable:ident, |$target:ident, $writer:ident| $closure:expr) => {
            {
                let content = $variable;
                let reader = Cursor::new(content.as_bytes());
                let mut $writer: Vec<u8> = Vec::new();
                let mut $target = TailState::from_slice(reader, &mut $writer).unwrap();
                $closure;
            }
        };
    }

    #[test]
    fn test_dump_to_tail() {
        tail_state_test!(CONTENT, |target, writer| {
            assert_eq!(target.dump_to_tail().is_ok(), true);
            assert_eq!(writer, CONTENT.as_bytes());
        })
    }

    #[test]
    fn test_dump_to_tail_without_line_ending() {
        tail_state_test!(CONTENT_WITHOUT_LINE_ENDING, |target, writer| {
            assert_eq!(target.dump_to_tail().is_ok(), true);
            assert_eq!(writer, CONTENT_WITHOUT_LINE_ENDING.as_bytes());
        })
    }

    #[test]
    fn test_tail() {
        tail_state_test!(CONTENT, |target, writer| {
            let result = tail_from_reader(&mut target, 1);
            assert_eq!(result.is_ok(), true);
            assert_eq!(writer, "line5\n".as_bytes());
        })
    }

    #[test]
    fn test_tail_without_line_ending() {
        tail_state_test!(CONTENT_WITHOUT_LINE_ENDING, |target, writer| {
            let result = tail_from_reader(&mut target, 1);
            assert_eq!(result.is_ok(), true);
            assert_eq!(writer, "line5".as_bytes());
        })
    }
}
