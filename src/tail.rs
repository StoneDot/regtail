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
use std::io::{self, Read, Seek, SeekFrom, Write, Result};
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

pub struct SeekableReader<T> where
    T: Read + Seek + Length {
    reader: T,
    seek_pos: u64,
}

impl SeekableReader<std::fs::File> {
    pub fn from_file(mut file: File) -> Result<SeekableReader<File>> {
        let pos = file.seek(SeekFrom::Current(0))?;
        Ok(SeekableReader {
            reader: file,
            seek_pos: pos,
        })
    }
}

impl<T> SeekableReader<T> where
    T: Read + Seek + Length {
    pub fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        self.reader.read(&mut buf)
    }

    pub fn seek(mut self: &mut Self, seek: SeekFrom) -> Result<u64> {
        self.seek_pos = self.reader.seek(seek)?;
        Ok(self.seek_pos)
    }

    pub fn current_seek(self: &Self) -> u64 {
        self.seek_pos
    }

    pub fn len(self: &Self) -> Result<u64> {
        self.reader.len()
    }

    fn tail_start_position(self: &mut SeekableReader<T>, tail_count: u64) -> Result<u64> {
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

    pub fn handle_shrink(self: &mut SeekableReader<T>, offset: u64) -> Result<bool> {
        let len = self.len()?;
        if len < offset {
            self.seek(SeekFrom::Start(0))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn seek_to_initial_position(self: &mut SeekableReader<T>, offset: u64) -> Result<u64> {
        // Shrink handling
        if self.handle_shrink(offset)? {
            return Ok(0);
        }

        // Seek to target position
        self.seek(SeekFrom::Start(offset))
    }

    pub fn dump_to_tail(self: &mut SeekableReader<T>) -> Result<u64> {
        let mut buffer = [0; BUFFER_SIZE];
        let mut offset = self.current_seek();
        let initial_size = (BUFFER_LEN - (offset % BUFFER_LEN)) as usize;
        let mut target = &mut buffer[..initial_size];

        // Read initial data
        let read_size = self.read(&mut target)?;
        target = &mut target[..read_size];
        offset += read_size as u64;

        if read_size == 0 {
            return Ok(offset);
        } else {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            loop {
                // Write to stdio
                lock.write(&target)?;

                // Read additional data
                let read_size = self.read(&mut buffer)?;
                target = &mut buffer[..read_size];
                offset += read_size as u64;
                if read_size == 0 {
                    return Ok(offset);
                }
            }
        }
    }
}

pub fn tail(path: &PathBuf, tail_count: u64) -> Result<SeekableReader<File>> {
    let file = File::open(path)?;
    let mut reader = SeekableReader::from_file(file)?;
    let offset = reader.tail_start_position(tail_count)?;
    reader.seek_to_initial_position(offset)?;
    reader.dump_to_tail()?;
    Ok(reader)
}