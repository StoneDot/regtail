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
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

// Max recommended buffer size is 128kB
// We choose reasonable size 8kB
const BUFFER_SIZE: usize = 8 * 1024;
const BUFFER_LEN: u64 = BUFFER_SIZE as u64;

fn tail_start_position(file: &mut File, tail_count: usize) -> Result<u64, io::Error> {
    let mut buffer = [0u8; BUFFER_SIZE];

    // Read file from tail requires file size
    let meta = file.metadata()?;

    // Empty file consideration
    if meta.len() == 0 {
        return Ok(0);
    }

    // Skip EOS
    let end_index = meta.len() - 1;
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
    file.seek(SeekFrom::Start(start_index))?;
    let mut read_size = file.read(&mut buffer)?;

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
        file.seek(SeekFrom::Start(start_index))?;
        read_size = file.read(&mut buffer)?;
        target = &buffer[..read_size];
    }
}

pub fn handle_shrink(file: &mut File, offset: u64) -> Result<bool, std::io::Error> {
    let file_size = file.metadata()?.len();
    if file_size < offset {
        file.seek(SeekFrom::Start(0))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn seek_to_initial_position(mut file: &mut File, offset: u64) -> Result<u64, std::io::Error> {
    // Shrink handling
    if handle_shrink(&mut file, offset)? {
        return Ok(0);
    }

    // Seek to target position
    file.seek(SeekFrom::Start(offset))
}

pub fn dump_to_tail(file: &mut File) -> Result<u64, std::io::Error> {
    let mut buffer = [0; BUFFER_SIZE];
    let mut offset = file.seek(SeekFrom::Current(0))?;
    let initial_size = (BUFFER_LEN - (offset % BUFFER_LEN)) as usize;
    let mut target = &mut buffer[..initial_size];

    // Read initial data
    let read_size = file.read(&mut target)?;
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
            let read_size = file.read(&mut buffer)?;
            target = &mut buffer[..read_size];
            offset += read_size as u64;
            if read_size == 0 {
                return Ok(offset);
            }
        }
    }
}

pub fn tail(path: &PathBuf, tail_count: usize) -> Result<File, std::io::Error> {
    let mut file = File::open(path)?;
    let offset = tail_start_position(&mut file, tail_count)?;
    seek_to_initial_position(&mut file, offset)?;
    dump_to_tail(&mut file)?;
    Ok(file)
}