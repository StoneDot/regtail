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

use std::path::PathBuf;
use std::str::FromStr;

use notify::RecursiveMode;
use structopt::StructOpt;

lazy_static! {
    static ref CURRENT_DIR: PathBuf = {
        PathBuf::from_str(".").unwrap()
    };
}

// For command line arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "regtail")]
pub struct Opt {
    /// Lines to show
    #[structopt(short = "l", long = "lines", default_value = "10")]
    pub lines: usize,

    /// Enable recursive watch
    #[structopt(short = "r", long = "recursive")]
    pub recursive: bool,

    /// Maximum recursive depth
    #[structopt(short = "d", long = "depth")]
    depth: Option<usize>,

    /// Target directory to process
    #[structopt(short = "p", long = "path", parse(from_os_str))]
    path: Option<PathBuf>,

    /// Regex to filter target files
    #[structopt(name = "REGEX")]
    pub regex: Option<String>,
}

impl Opt {
    pub fn recursive_mode(self: &Opt) -> RecursiveMode {
        if self.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        }
    }

    pub fn watch_path(self: &Opt) -> &PathBuf {
        match &self.path {
            Some(path) => path,
            None => &CURRENT_DIR,
        }
    }

    pub fn watch_path_is_dir(self: &Opt) -> bool {
        if let Some(path) = &self.path {
            return path.is_dir();
        }
        true
    }

    pub fn depth(self: &Opt) -> Option<usize> {
        if self.recursive {
            self.depth
        } else {
            Some(1)
        }
    }
}