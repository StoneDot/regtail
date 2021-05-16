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
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufWriter, ErrorKind, Stdout};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::channel;

use ansi_term::Colour::Blue;
use lru::LruCache;
use notify::{op::Op, raw_watcher, Error as NotifyError, RawEvent, Watcher};
use pathdiff::diff_paths;

use crate::tail::{CachedTailState, SeekPos};

use super::filter::PathFilter;
use super::tail::{tail2, FileReader, FileRepository, Length, TailState};
use super::Opt;

const MAX_FILE_HANDLE: usize = 512;

pub struct DirectoryWatcher<T, U>
where
    T: std::io::Read + std::io::Seek + SeekPos + Length,
    U: std::io::Write,
{
    filter: PathFilter,
    current_dir: Option<PathBuf>,
    selected_file_path: Option<PathBuf>,
    file_map: HashMap<PathBuf, CachedTailState>,
    renaming_map: HashMap<u32, Option<TailState<T, U>>>,
    repository: FileRepository,
    colorize: bool,
}

impl DirectoryWatcher<FileReader, BufWriter<Stdout>> {
    pub fn new(opt: &Opt) -> Result<DirectoryWatcher<FileReader, BufWriter<Stdout>>, i32> {
        // Check whether supplied path is a directory
        if !opt.watch_path_is_dir() {
            eprintln!("supplied path is not a directory");
            return Err(1);
        }

        // Generate filter
        let filter = PathFilter::new(&opt)?;

        // Retrieve current directory
        let current_dir = std::env::current_dir().ok();

        let repository: FileRepository = Rc::new(RefCell::new(LruCache::new(MAX_FILE_HANDLE)));

        Ok(DirectoryWatcher {
            filter,
            current_dir,
            selected_file_path: None,
            file_map: HashMap::new(),
            renaming_map: HashMap::new(),
            repository,
            colorize: opt.colorize,
        })
    }
}

impl DirectoryWatcher<FileReader, BufWriter<Stdout>> {
    fn print_normalized_path(&self, path: &Path) {
        let relative_path = path.to_string_lossy();
        let display_path = relative_path.trim_start_matches("./");

        if self.colorize {
            print!("{}", Blue.bold().paint("==> "));
            self.filter.print_path_with_color(display_path);
            println!("{}", Blue.bold().paint(" <=="));
        } else {
            println!("==> {} <==", display_path);
        }
    }

    fn normalize_path_for_windows(canonical_path: PathBuf) -> PathBuf {
        if cfg!(target_os = "windows") {
            let lossy_str = canonical_path.to_string_lossy();
            let path = lossy_str.replace("/", "\\");
            if path.starts_with("\\\\?\\") {
                let mut path = path.trim_start_matches("\\\\?\\");
                if path.starts_with("UNC\\") {
                    path = path.trim_start_matches("UNC\\");
                }
                return PathBuf::from(path);
            }
            return PathBuf::from(path);
        }
        canonical_path
    }

    fn canonicalize_path(path: &Path) -> io::Result<PathBuf> {
        let canonical_path = path.canonicalize()?;
        Ok(Self::normalize_path_for_windows(canonical_path))
    }

    fn pending_delete_file(path: &Path) -> bool {
        if let Err(e) = File::open(path) {
            if e.kind() == ErrorKind::PermissionDenied {
                return true;
            }
        }
        false
    }

    fn handle_pending_delete(&mut self, pending_delete_files: &mut VecDeque<PathBuf>) {
        // On Windows, try to detect pending delete files
        if cfg!(target_os = "windows") {
            for path in self.file_map.keys() {
                if Self::pending_delete_file(path) {
                    pending_delete_files.push_back(path.to_owned());
                }
            }
            // Release file handles with pending delete to ensure actually they're deleted
            for path in pending_delete_files.iter() {
                {
                    let mut repo = (*self.repository).borrow_mut();
                    repo.pop(path);
                }
                if let Some(reader) = self.file_map.remove(path) {
                    self.unsubscribe_select_file(path, &reader);
                }
            }
            pending_delete_files.clear();
        }
    }

    fn print_file_path(&self, path: &Path) {
        let mut preceding = "\n";
        if let Some(selected_file_path) = &self.selected_file_path {
            if let Some(selected_file) = self.file_map.get(selected_file_path) {
                if !selected_file.printed_eol() {
                    println!();
                }
            }
        } else {
            preceding = "";
        }
        if let Some(current_dir) = &self.current_dir {
            if let Some(relative_path) = diff_paths(&path, &current_dir) {
                print!("{}", preceding);
                self.print_normalized_path(&relative_path);
                return;
            }
        }
        print!("{}", preceding);
        self.print_normalized_path(path);
    }

    fn unsubscribe_select_file(&mut self, path: &Path, reader: &CachedTailState) {
        if let Some(selected_file_path) = &self.selected_file_path {
            if selected_file_path == path {
                if !reader.printed_eol() {
                    println!();
                }
                println!();
                self.selected_file_path = None
            }
        }
    }

    fn change_selected_file(&mut self, path: &Path) {
        // Handle current path change
        if let Some(last_path) = &self.selected_file_path {
            if last_path != path {
                self.print_file_path(&path);
                self.selected_file_path = Some(path.to_owned());
            }
        } else {
            // Should print file path because of first output of the program
            self.print_file_path(&path);
            self.selected_file_path = Some(path.to_owned());
        }
    }

    fn handle_write(&mut self, path: PathBuf) -> std::io::Result<()> {
        // Just ignore if the path is not match regex
        if !self.filter.match_path(&path) {
            return Ok(());
        }

        self.change_selected_file(&path);

        match self.file_map.get_mut(&path) {
            Some(reader) => {
                // Shrink handling
                let offset = reader.current_seek();
                reader.handle_shrink(offset)?;
                reader.dump_to_tail()?;
            }
            None => {
                // Check file existence
                if !Path::exists(&path) {
                    return Ok(());
                }

                // Supplied path is not opened currently
                let mut reader =
                    CachedTailState::from_path(path.clone(), Rc::clone(&self.repository))?;
                reader.dump_to_tail()?;
                self.file_map.insert(path, reader);
            }
        }
        Ok(())
    }

    #[allow(clippy::single_match)]
    fn handle_rename(&mut self, path: PathBuf, cookie: Option<u32>) {
        if let Some(cookie) = cookie {
            match self.renaming_map.remove(&cookie) {
                Some(file) => match file {
                    Some(file) => {
                        // Just ignore if the new path is not match regex
                        if !self.filter.match_path(&path) {
                            return;
                        }

                        // New path supplied
                        self.file_map.insert(path, file);
                    }
                    None => {
                        // This is maybe duplication request
                        // Just ignore
                    }
                },
                None => {
                    // Old path supplied
                    match self.file_map.remove(&path) {
                        Some(file) => {
                            self.unsubscribe_select_file(&path, &file);
                            self.renaming_map.insert(cookie, Some(file));
                        }
                        None => {
                            self.renaming_map.insert(cookie, None);
                        }
                    }
                }
            }
        }
    }

    fn handle_remove(&mut self, path: &PathBuf) {
        if let Some(reader) = self.file_map.remove(path) {
            {
                let mut repo = (*self.repository).borrow_mut();
                repo.pop(path);
            }
            self.unsubscribe_select_file(path, &reader);
        }
    }

    pub fn follow_dir(&mut self, opt: &Opt) -> Result<(), NotifyError> {
        // Empty tailing consideration
        if opt.lines == 0 {
            for path in self.filter.filtered_files(&opt) {
                let canonical_path = Self::canonicalize_path(&path)?;
                let reader = tail2(
                    PathBuf::from(&canonical_path),
                    Rc::clone(&self.repository),
                    0,
                )?;
                self.file_map.insert(canonical_path.to_owned(), reader);
            }
        } else {
            let mut prev_reader: Option<&CachedTailState> = None;
            for path in self.filter.filtered_files(&opt) {
                if self.selected_file_path.is_some() {
                    // If there is a previous file and its last byte is not \n,
                    // put \n for consistent result.
                    if let Some(reader) = prev_reader {
                        if !reader.printed_eol() {
                            println!();
                        }
                    }

                    println!();
                }
                self.print_normalized_path(&path);
                let canonical_path = Self::canonicalize_path(&path)?;
                let reader = tail2(
                    PathBuf::from(&canonical_path),
                    Rc::clone(&self.repository),
                    opt.lines,
                )?;

                self.file_map.insert(canonical_path.to_owned(), reader);
                prev_reader = Some(&self.file_map[&canonical_path]);
                self.selected_file_path = Some(canonical_path);
            }
        }

        let (tx, rx) = channel();
        let mut watcher = raw_watcher(tx)?;
        let watch_path = opt.watch_path();
        let recursive_mode = opt.recursive_mode();
        watcher.watch(watch_path.as_os_str(), recursive_mode)?;

        let mut pending_delete_files = VecDeque::new();
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(RawEvent {
                    path: Some(mut path),
                    op: Ok(op),
                    cookie,
                }) => {
                    path = Self::normalize_path_for_windows(path);

                    // On MacOS, some simultaneous operation cannot handle correctly.
                    // This is why the curious handling is required.
                    if cfg!(target_os = "macos") {
                        // FSEvents cannot handle renaming and other operations simultaneously.
                        if op.contains(Op::RENAME) && cookie.is_some() {
                            // Try to handle renaming correctly at the sacrifice of other operations.
                            self.handle_rename(path.to_owned(), cookie);
                        } else {
                            // Renaming and removing may not happen same time.
                            // Therefore in the case of Op = REMOVE | RENAME,
                            // just ignore remove operation to consider REMOVE is stale.
                            if op.contains(Op::REMOVE) && !op.contains(Op::RENAME) {
                                self.handle_remove(&path)
                            }
                            if op.contains(Op::WRITE) {
                                self.handle_write(path)?
                            }
                        }
                    } else {
                        // Except for Mac OS, op can be treated as atomic
                        if op == Op::WRITE {
                            self.handle_write(path)?
                        } else if op == Op::REMOVE {
                            self.handle_remove(&path)
                        } else if op == Op::RENAME {
                            self.handle_rename(path, cookie);
                        }
                    }
                }
                Ok(event) => {
                    return Err(NotifyError::Generic(format!("broken event: {:?}", event)));
                }
                Err(e) => {
                    if e == std::sync::mpsc::RecvTimeoutError::Disconnected {
                        return Err(NotifyError::Generic(format!("watch error: {:?}", e)));
                    }
                }
            }
            self.handle_pending_delete(&mut pending_delete_files);
        }
    }
}
