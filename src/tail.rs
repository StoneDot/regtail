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

use std::cell::RefCell;
use std::cmp::max;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, sink, Read, Result, Seek, SeekFrom, Sink, Stdout, Write};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use lru::LruCache;

// Max recommended buffer size is 128kB
// We choose reasonable size 8kB
const BUFFER_SIZE: usize = 8 * 1024;
const BUFFER_LEN: u64 = BUFFER_SIZE as u64;

pub type FileRepository = Rc<RefCell<LruCache<PathBuf, Rc<RefCell<File>>>>>;
pub type FileReader = TransparentReader<PathBuf, File, FileCreator>;
pub type CachedTailState = TailState<FileReader, io::BufWriter<Stdout>>;

pub trait ReaderCreator<K, T> {
    fn create_reader(&self, path: &K) -> Result<T>;
}

pub struct FileCreator;

impl ReaderCreator<PathBuf, File> for FileCreator {
    fn create_reader(&self, path: &PathBuf) -> Result<File> {
        File::open(path)
    }
}
pub struct TransparentReader<K, T, C>
where
    K: Hash + Eq + Clone,
    T: Read + Seek + Length,
    C: ReaderCreator<K, T>,
{
    reader_repository: Rc<RefCell<LruCache<K, Rc<RefCell<T>>>>>,
    path: K,
    reader_seek_pos: u64,
    reader_cache: RefCell<Weak<RefCell<T>>>,
    reader_creator: C,
}

impl<K, T, C> TransparentReader<K, T, C>
where
    K: Hash + Eq + Clone,
    T: Read + Seek + Length,
    C: ReaderCreator<K, T>,
{
    fn reader(&self) -> Result<Rc<RefCell<T>>> {
        let mut reader_cache = self.reader_cache.borrow_mut();
        let reader = reader_cache.upgrade();
        if let Some(x) = reader {
            return Ok(x);
        }
        let mut reader_repo = (*self.reader_repository).borrow_mut();
        match reader_repo.get(&self.path) {
            Some(reader) => Ok(Rc::clone(reader)),
            None => {
                let file = self.reader_creator.create_reader(&self.path)?;
                reader_repo.put(self.path.clone(), Rc::new(RefCell::new(file)));
                let data = reader_repo.get(&self.path).unwrap();
                *reader_cache = Rc::downgrade(data);
                Ok(Rc::clone(data))
            }
        }
    }
}

impl<K, T, C> Read for TransparentReader<K, T, C>
where
    K: Hash + Eq + Clone,
    T: Read + Seek + Length,
    C: ReaderCreator<K, T>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let rc_reader = self.reader()?;
        let mut reader = (*rc_reader).borrow_mut();
        let size = (*reader).read(buf);
        if let Ok(size) = size {
            self.reader_seek_pos += size as u64
        }
        size
    }
}

impl<K, T, C> Seek for TransparentReader<K, T, C>
where
    K: Hash + Eq + Clone,
    T: Read + Seek + Length,
    C: ReaderCreator<K, T>,
{
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let rc_reader = self.reader()?;
        let mut reader = (*rc_reader).borrow_mut();
        if let SeekFrom::Current(_) = pos {
            reader.seek(SeekFrom::Start(self.reader_seek_pos))?;
            let seek_pos = reader.seek(pos);
            if let Ok(pos) = seek_pos {
                self.reader_seek_pos = pos
            }
            seek_pos
        } else {
            let seek_pos = reader.seek(pos);
            if let Ok(pos) = seek_pos {
                self.reader_seek_pos = pos
            }
            seek_pos
        }
    }
}

impl<K, T, C> Length for TransparentReader<K, T, C>
where
    K: Hash + Eq + Clone,
    T: Read + Seek + Length,
    C: ReaderCreator<K, T>,
{
    fn len(&self) -> Result<u64> {
        let rc_reader = self.reader()?;
        let reader = (*rc_reader).borrow_mut();
        reader.len()
    }
}

impl<K, T, C> SeekPos for TransparentReader<K, T, C>
where
    K: Hash + Eq + Clone,
    T: Read + Seek + Length,
    C: ReaderCreator<K, T>,
{
    fn seek_pos(&self) -> u64 {
        self.reader_seek_pos
    }
}

impl TransparentReader<PathBuf, File, FileCreator> {
    fn new(
        path: PathBuf,
        repository: Rc<RefCell<LruCache<PathBuf, Rc<RefCell<File>>>>>,
    ) -> TransparentReader<PathBuf, File, FileCreator> {
        TransparentReader {
            reader_repository: repository,
            path,
            reader_seek_pos: 0,
            reader_cache: RefCell::new(Weak::new()),
            reader_creator: FileCreator,
        }
    }
}

// Allow lack of is_empty function because of len returns Result type
#[allow(clippy::len_without_is_empty)]
pub trait Length {
    fn len(&self) -> Result<u64>;
}

pub trait SeekPos {
    fn seek_pos(&self) -> u64;
}

impl Length for File {
    fn len(&self) -> Result<u64> {
        Ok(self.metadata()?.len())
    }
}

pub struct TailState<T, U>
where
    T: Read + Seek + SeekPos + Length,
    U: Write,
{
    reader: T,
    writer: U,
    printed_eol: bool,
}

impl CachedTailState {
    pub fn from_path(path: PathBuf, repo: FileRepository) -> Result<CachedTailState> {
        let reader = FileReader::new(path, repo);
        Self::from_file_reader(reader)
    }

    pub fn from_file_reader(reader: FileReader) -> Result<CachedTailState> {
        let writer = io::BufWriter::new(io::stdout());
        Ok(CachedTailState {
            reader,
            writer,
            printed_eol: false,
        })
    }
}

pub struct DirectFileReader {
    file: File,
    reader_seek_pos: u64,
}

impl DirectFileReader {
    #[allow(dead_code)]
    pub fn new(path: &Path) -> io::Result<DirectFileReader> {
        let file = File::open(path)?;
        Ok(DirectFileReader {
            file,
            reader_seek_pos: 0,
        })
    }
}

impl Read for DirectFileReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let size = self.file.read(buf)?;
        self.reader_seek_pos += size as u64;
        Ok(size)
    }
}

impl Seek for DirectFileReader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let seek_pos = self.file.seek(pos)?;
        self.reader_seek_pos = seek_pos;
        Ok(seek_pos)
    }
}

impl SeekPos for DirectFileReader {
    fn seek_pos(&self) -> u64 {
        self.reader_seek_pos
    }
}

impl Length for DirectFileReader {
    fn len(&self) -> Result<u64> {
        Ok(self.file.metadata()?.len())
    }
}

#[allow(dead_code)]
pub fn from_file_to_sink(path: &Path) -> io::Result<TailState<DirectFileReader, Sink>> {
    Ok(TailState {
        reader: DirectFileReader::new(path)?,
        writer: sink(),
        printed_eol: false,
    })
}

// Allow lack of is_empty function because of len returns Result type
#[allow(clippy::len_without_is_empty)]
impl<T, U> TailState<T, U>
where
    T: Read + Seek + SeekPos + Length,
    U: Write,
{
    pub fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        self.reader.read(&mut buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(&buf)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }

    pub fn seek(&mut self, seek: SeekFrom) -> Result<u64> {
        self.reader.seek(seek)
    }

    pub fn current_seek(&self) -> u64 {
        self.reader.seek_pos()
    }

    pub fn len(&self) -> Result<u64> {
        self.reader.len()
    }

    pub fn printed_eol(&self) -> bool {
        self.printed_eol
    }

    fn tail_start_position(&mut self, tail_count: u64) -> Result<u64> {
        let mut buffer = [0u8; BUFFER_SIZE];

        // Read file from tail requires file size
        let len = self.len()?;

        // Empty file consideration
        if len == 0 {
            return Ok(0);
        }

        // Empty tailing consideration
        if tail_count == 0 {
            let pos = self.seek(SeekFrom::End(0))?;
            return Ok(pos);
        }

        // Skip EOS
        let end_index = len - 1;
        if end_index == 0 {
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
            start_index -= BUFFER_LEN;
            self.seek(SeekFrom::Start(start_index))?;
            read_size = self.read(&mut buffer)?;
            target = &buffer[..read_size];
        }
    }

    pub fn handle_shrink(&mut self, offset: u64) -> Result<bool> {
        let len = self.len()?;
        if len < offset {
            self.seek(SeekFrom::Start(0))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn seek_with_shrink_handling(&mut self, offset: u64) -> Result<u64> {
        // Shrink handling
        if self.handle_shrink(offset)? {
            return Ok(0);
        }

        // Seek to target position
        self.seek(SeekFrom::Start(offset))
    }

    pub fn dump_to_tail(&mut self) -> Result<u64> {
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
            Ok(offset)
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

pub fn tail_from_reader<T, U>(reader: &mut TailState<T, U>, tail_count: u64) -> Result<u64>
where
    T: Read + Seek + SeekPos + Length,
    U: Write,
{
    let offset = reader.tail_start_position(tail_count)?;
    reader.seek_with_shrink_handling(offset)?;
    reader.dump_to_tail()
}

pub fn tail2(path: PathBuf, repo: FileRepository, tail_count: u64) -> Result<CachedTailState> {
    let mut tail_state = CachedTailState::from_path(path, repo)?;
    let _offset = tail_from_reader(&mut tail_state, tail_count);
    Ok(tail_state)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::io::Result;

    use super::tail_from_reader;
    use super::Length;
    use super::TailState;
    use crate::tail::SeekPos;

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

    impl SeekPos for Cursor<&[u8]> {
        fn seek_pos(&self) -> u64 {
            self.get_ref().len() as u64
        }
    }

    impl TailState<Cursor<&[u8]>, &mut Vec<u8>> {
        pub fn from_slice<'a>(
            reader: Cursor<&'a [u8]>,
            writer: &'a mut Vec<u8>,
        ) -> Result<TailState<Cursor<&'a [u8]>, &'a mut Vec<u8>>> {
            Ok(TailState {
                reader,
                writer,
                printed_eol: false,
            })
        }
    }

    macro_rules! tail_state_test {
        ( $variable:ident, |$target:ident, $writer:ident| $closure:expr) => {{
            let content = $variable;
            let reader = Cursor::new(content.as_bytes());
            let mut $writer: Vec<u8> = Vec::new();
            let mut $target = TailState::from_slice(reader, &mut $writer).unwrap();
            $closure;
        }};
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
