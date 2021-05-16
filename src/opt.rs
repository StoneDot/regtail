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

use clap::{self, Arg};
use notify::RecursiveMode;

lazy_static! {
    static ref CURRENT_DIR: PathBuf = PathBuf::from_str(".").unwrap();
}

pub struct Opt {
    pub lines: u64,
    pub recursive: bool,
    pub show_binary: bool,
    depth: Option<usize>,
    pub regex: Option<String>,
    path: Option<PathBuf>,
    pub colorize: bool,
}

pub enum ParseError {
    ColorParseFailed,
}

impl Opt {
    pub fn generate() -> Result<Opt, ParseError> {
        let matches = app_from_crate!()
            .arg(
                Arg::with_name("recursive")
                    .short("r")
                    .long("recursive")
                    .help("Enable recursive watch"),
            )
            .arg(
                Arg::with_name("show-binary")
                    .long("show-binary")
                    .help("Enable binary tailing"),
            )
            .arg(
                Arg::with_name("regex")
                    .short("e")
                    .long("regex")
                    .help("Regex to filter target files")
                    .allow_hyphen_values(true)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("path")
                    .short("p")
                    .long("path")
                    .help("Target directory to process")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("depth")
                    .short("d")
                    .help("Maximum recursive depth")
                    .requires("recursive")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("lines")
                    .short("l")
                    .help("Lines to show")
                    .default_value("10")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("REGEX")
                    .help("Regex to filter target files")
                    .required(false)
                    .index(1)
                    .conflicts_with("regex")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("PATH")
                    .help("Target directory to process")
                    .required(false)
                    .index(2)
                    .conflicts_with("path")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("color")
                    .short("c")
                    .required(false)
                    .takes_value(true)
                    .possible_values(&["auto", "never", "always"])
                    .help("Colorize mode"),
            )
            .get_matches();
        let color_mode = matches.value_of("color").unwrap_or("auto");
        let colorize = match color_mode {
            "auto" => Ok(atty::is(atty::Stream::Stdout)),
            "never" => Ok(false),
            "always" => Ok(true),
            _ => Err(ParseError::ColorParseFailed),
        }?;
        Ok(Opt {
            lines: value_t!(matches, "lines", u64).unwrap_or_else(|e| e.exit()),
            recursive: matches.is_present("recursive"),
            show_binary: matches.is_present("show-binary"),
            depth: value_t!(matches.value_of("depth"), usize)
                .map(Some)
                .unwrap_or_else(|e| {
                    if e.kind == clap::ErrorKind::ArgumentNotFound {
                        None
                    } else {
                        e.exit()
                    }
                }),
            regex: matches
                .value_of("regex")
                .map(|x| x.to_owned())
                .or_else(|| matches.value_of("REGEX").map(|x| x.to_owned())),
            path: matches
                .value_of_os("path")
                .map(PathBuf::from)
                .or_else(|| matches.value_of_os("PATH").map(PathBuf::from)),
            colorize,
        })
    }

    pub fn recursive_mode(self: &Opt) -> RecursiveMode {
        if self.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        }
    }

    pub fn watch_path(self: &Opt) -> &PathBuf {
        self.path.as_ref().unwrap_or(&CURRENT_DIR)
    }

    pub fn watch_path_is_dir(self: &Opt) -> bool {
        self.watch_path().is_dir()
    }

    pub fn depth(self: &Opt) -> Option<usize> {
        if self.recursive {
            self.depth
        } else {
            Some(1)
        }
    }
}
