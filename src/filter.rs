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

use std::path::{Path, PathBuf};

use ansi_term::Colour::{Blue, Green};
use content_inspector::{inspect, ContentType};
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

use super::Opt;
use std::fs::File;
use std::io::Read;
use std::str::from_utf8;

const MAX_BUFFER_SIZE: usize = 1024;

pub struct PathFilter {
    regex: Regex,
    filter_binary: bool,
}

fn is_text(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut buf = [0u8; MAX_BUFFER_SIZE];
    let inspect_buf = match file.read(&mut buf) {
        Ok(size) => &buf[0..size],
        Err(_) => return false,
    };
    let file_type = inspect(inspect_buf);
    match file_type {
        ContentType::BINARY => false,
        ContentType::UTF_8 | ContentType::UTF_8_BOM => match from_utf8(inspect_buf) {
            Ok(_) => true,
            Err(e) => e.error_len().is_none(),
        },
        ContentType::UTF_16BE
        | ContentType::UTF_16LE
        | ContentType::UTF_32BE
        | ContentType::UTF_32LE => {
            // TODO: Need validation, currently not implemented
            true
        }
    }
}

impl PathFilter {
    pub fn new(opt: &Opt) -> Result<PathFilter, i32> {
        // Create regex filter
        let regex = match Self::generate_filter_regex(&opt) {
            Ok(regex) => regex,
            Err(error) => match error {
                regex::Error::Syntax(message) => {
                    eprintln!("invalid regex supplied:\n{}", message);
                    return Err(1);
                }
                regex::Error::CompiledTooBig(size) => {
                    eprintln!("too big regex: {}", size);
                    return Err(1);
                }
                regex::Error::__Nonexhaustive => {
                    eprintln!("unexpected regex supplied");
                    return Err(1);
                }
            },
        };

        Ok(PathFilter {
            regex,
            filter_binary: !opt.show_binary,
        })
    }

    fn generate_filter_regex(opt: &Opt) -> Result<Regex, regex::Error> {
        match &opt.regex {
            Some(regex) => Regex::new(regex),
            None => Regex::new(".*"),
        }
    }

    pub fn match_path(self: &PathFilter, path: &Path) -> bool {
        match path.to_str() {
            Some(path_str) => self.regex.is_match(path_str),
            None => false,
        }
    }

    pub fn filtered_files<'a>(
        self: &'a PathFilter,
        opt: &Opt,
    ) -> impl Iterator<Item = std::path::PathBuf> + 'a {
        let walk_path = opt.watch_path();
        let depth = opt.depth();
        let walker = WalkDir::new(&walk_path).sort_by(|l, r| l.path().cmp(r.path()));
        let walker = match depth {
            Some(depth) => walker.max_depth(depth),
            None => walker,
        };
        walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(move |e: DirEntry| {
                let path = e.path();
                if !path.is_file() {
                    return None;
                }
                if self.match_path(&path) {
                    Some(path.to_owned())
                } else {
                    None
                }
            })
            .filter(move |path: &PathBuf| {
                if self.filter_binary {
                    is_text(path)
                } else {
                    true
                }
            })
    }

    pub fn print_path_with_color(self: &Self, path: &str) {
        let mut prev_end_point = 0;
        for m in self.regex.find_iter(path) {
            let prev_str = &path[prev_end_point..m.start()];
            print!("{}", Blue.bold().paint(prev_str));
            print!("{}", Green.bold().paint(m.as_str()));
            prev_end_point = m.end();
        }
        let len = path.len();
        let last_str = &path[prev_end_point..len];
        print!("{}", Blue.bold().paint(last_str));
    }
}
