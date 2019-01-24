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

use std::path::Path;

use regex::Regex;
use walkdir::{DirEntry, WalkDir};

use super::Opt;

pub struct PathFilter {
    regex: Regex,
}

impl PathFilter {
    pub fn new(opt: &Opt) -> Result<PathFilter, i32> {
        // Create regex filter
        let regex = match Self::generate_filter_regex(&opt) {
            Ok(regex) => regex,
            Err(error) => {
                match error {
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
                }
            }
        };

        Ok(PathFilter {
            regex
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
            None => false
        }
    }

    pub fn filtered_files<'a>(self: &'a PathFilter, opt: &Opt) -> impl Iterator<Item=std::path::PathBuf> + 'a {
        let walk_path = opt.watch_path();
        let depth = opt.depth();
        let walker = WalkDir::new(&walk_path)
            .sort_by(|l, r| l.path().cmp(r.path()));
        let walker = match depth {
            Some(depth) => walker.max_depth(depth),
            None => walker,
        };
        walker.into_iter()
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
    }
}