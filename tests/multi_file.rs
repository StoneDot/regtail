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


use std::process::Command;
use std::thread;
use std::time::Duration;
use thread::sleep;

use utils::RunningCommand;
use utils::WorkingDir;
use utils::KillStatus;

#[macro_use]
mod macros;
mod utils;

const WAIT_TIME: Duration = Duration::from_millis(50);

test!(multi_file_with_eol, |dir: WorkingDir, mut cmd: Command| {
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.put_file("file1", "test1!\n");
    sleep(WAIT_TIME);
    dir.put_file("file2", "test2!\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert!(output.contains("file1"));
    assert!(output.contains("file2"));
    assert!(output.contains(" <==\ntest1!\n\n==>"));
    assert!(output.contains(" <==\ntest2!\n"));
});

test!(multi_file_without_eol, |dir: WorkingDir, mut cmd: Command| {
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.put_file("file1", "test1!");
    sleep(WAIT_TIME);
    dir.put_file("file2", "test2!");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert!(output.contains("file1"));
    assert!(output.contains("file2"));
    assert!(output.contains(" <==\ntest1!\n\n==>"));
    assert!(output.contains(" <==\ntest2!"));
});

test!(multi_file_alread_exist_file, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file1", "test1!\n");
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("file1", "test2!\n");
    sleep(WAIT_TIME);
    dir.put_file("file2", "test3!\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert!(output.contains("file1"));
    assert!(output.contains("file2"));
    assert!(output.contains("file1 <==\ntest1!\ntest2!\n"));
    assert!(output.contains("file2 <==\ntest3!\n"));
});