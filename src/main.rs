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

#[macro_use]
extern crate lazy_static;

use structopt::StructOpt;

use opt::Opt;
use watcher::DirectoryWatcher;

mod filter;
mod opt;
mod watcher;
mod tail;

fn follow(opt: &Opt) -> i32 {
    let mut watcher = match DirectoryWatcher::new(&opt) {
        Ok(watcher) => watcher,
        Err(exit_status) => return exit_status,
    };
    match watcher.follow_dir(&opt) {
        Ok(exit_status) => {
            return exit_status;
        }
        Err(error) => {
            match error {
                notify::Error::Generic(string) => {
                    eprintln!("generic error: {}", string);
                }
                notify::Error::Io(error) => {
                    eprintln!("io error: {}", error);
                }
                notify::Error::PathNotFound => {
                    eprintln!("path not found");
                }
                notify::Error::WatchNotFound => {
                    eprintln!("watch not found");
                }
            }
            return 1;
        }
    }
}

fn app() -> i32 {
    let opt = Opt::from_args();
    follow(&opt)
}

fn main() {
    std::process::exit(app())
}
