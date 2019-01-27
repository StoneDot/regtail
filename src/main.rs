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

const EX_ERR: i32 = 1;
const EX_NOINPUT: i32 = 66;
const EX_SOFTWARE: i32 = 70;
const EX_IOERR: i32 = 74;

fn follow(opt: &Opt) -> Result<(), i32> {
    let mut watcher = DirectoryWatcher::new(&opt)?;
    watcher.follow_dir(&opt).map_err(|error| {
        match error {
            notify::Error::Generic(string) => {
                eprintln!("generic error: {}", string);
                EX_ERR
            }
            notify::Error::Io(error) => {
                eprintln!("io error: {}", error);
                EX_IOERR
            }
            notify::Error::PathNotFound => {
                eprintln!("path not found");
                EX_NOINPUT
            }
            notify::Error::WatchNotFound => {
                eprintln!("watch not found");
                EX_SOFTWARE
            }
        }
    })
}

fn app() -> i32 {
    let opt = Opt::from_args();
    follow(&opt).err().unwrap_or(0)
}

fn main() {
    std::process::exit(app())
}
