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

use std::collections::HashMap;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::channel;

use notify::{Error as NotifyError, op::Op, raw_watcher, RawEvent, Watcher};
use pathdiff::diff_paths;

use super::Opt;
use super::filter::PathFilter;
use super::tail::{tail, dump_to_tail, handle_shrink};

pub struct DirectoryWatcher {
    filter: PathFilter,
    current_dir: PathBuf,
    current_path: Option<PathBuf>,
    file_map: HashMap<PathBuf, File>,
    renaming_map: HashMap<u32, File>,
}

impl DirectoryWatcher {
    pub fn new(opt: &Opt) -> Result<DirectoryWatcher, i32> {
        // Check whether supplied path is a directory
        if !opt.watch_path_is_dir() {
            eprintln!("supplied path is not a directory");
            return Err(1);
        }

        // Generate filter
        let filter = PathFilter::new(&opt)?;

        // Retrieve current directory
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(_) => return Err(1),
        };

        Ok(DirectoryWatcher {
            filter,
            current_dir,
            current_path: None,
            file_map: HashMap::new(),
            renaming_map: HashMap::new(),
        })
    }

    fn print_file_path(self: &DirectoryWatcher, path: &PathBuf) {
        match diff_paths(&path, &self.current_dir) {
            Some(relative_path) => {
                println!("\n==> {} <==", relative_path.display())
            }
            None => {
                println!("\n==> {} <==", path.display())
            }
        };
    }

    fn change_current_path(self: &mut DirectoryWatcher, path: &PathBuf) {
        // Handle current path change
        if let Some(last_path) = &self.current_path {
            if last_path != path {
                self.print_file_path(&path);
                self.current_path = Some(path.to_owned());
            }
        }
    }

    fn handle_write(self: &mut DirectoryWatcher, path: PathBuf) -> std::io::Result<()> {
        // Just ignore if the path is not match regex
        if !self.filter.match_path(&&path) {
            return Ok(());
        }

        self.change_current_path(&path);

        match self.file_map.get_mut(&path) {
            Some(mut file) => {
                // Shrink handling
                let offset = file.seek(SeekFrom::Current(0))?;
                handle_shrink(&mut file, offset)?;
                dump_to_tail(&mut file)?;
            }
            None => {
                // Supplied path is not opened currently
                let mut file = File::open(&path)?;
                dump_to_tail(&mut file)?;
                self.file_map.insert(path, file);
            }
        }
        Ok(())
    }

    fn handle_create(self: &mut DirectoryWatcher, path: PathBuf) -> std::io::Result<()> {
        // Just ignore if the path is not match regex
        if !self.filter.match_path(&path) {
            return Ok(());
        }

        // Newly created file should be dumped first and watched
        self.file_map.remove(&path);
        let mut file = File::open(&path)?;
        self.change_current_path(&path);
        dump_to_tail(&mut file)?;
        self.file_map.insert(path, file);
        Ok(())
    }

    fn handle_rename(self: &mut DirectoryWatcher, path: PathBuf, cookie: Option<u32>) {
        match self.renaming_map.remove(&cookie.unwrap()) {
            Some(file) => {
                // Just ignore if the new path is not match regex
                if !self.filter.match_path(&path) {
                    return;
                }

                // New path supplied
                self.file_map.insert(path, file);
            }
            None => {
                // Old path supplied
                if let Some(file) = self.file_map.remove(&path) {
                    self.renaming_map.insert(cookie.unwrap(), file);
                }
            }
        }
    }

    pub fn follow_dir(self: &mut DirectoryWatcher, opt: &Opt) -> Result<i32, NotifyError> {
        let (tx, rx) = channel();
        let mut watcher = raw_watcher(tx)?;

        for path in self.filter.filtered_files(&opt) {
            if self.current_path.is_some() {
                println!();
            }
            println!("==> {} <==", path.display());
            let file = tail(&PathBuf::from(&path), opt.lines)?;
            let canonical_path = path.canonicalize()?;
            self.file_map.insert(canonical_path.to_owned(), file);
            self.current_path = Some(canonical_path);
        }

        let watch_path = opt.watch_path();

        loop {
            let recursive_mode = opt.recursive_mode();
            watcher.watch(watch_path.as_os_str(), recursive_mode)?;
            match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(RawEvent { path: Some(path), op: Ok(op), cookie }) => {
                    if op == Op::WRITE {
                        self.handle_write(path)?
                    } else if op == Op::CREATE {
                        self.handle_create(path)?
                    } else if op == Op::REMOVE {
                        self.file_map.remove(&path);
                    } else if op == Op::RENAME {
                        self.handle_rename(path, cookie);
                    }
                }
                Ok(event) => {
                    eprintln!("broken event: {:?}", event);
                    return Ok(1);
                }
                Err(e) => {
                    if e == std::sync::mpsc::RecvTimeoutError::Disconnected {
                        eprintln!("watch error: {:?}", e);
                        return Ok(1);
                    }
                }
            }
        }
    }
}