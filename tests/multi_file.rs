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

use utils::KillStatus;
use utils::RunningCommand;
use utils::WorkingDir;

#[macro_use]
mod macros;
mod utils;

const WAIT_TIME: Duration = Duration::from_millis(200);
const RENAME_WAIT_TIME: Duration = Duration::from_millis(1000);

fn sleep_for_rename() {
    if cfg!(target_os = "macos") {
        sleep(RENAME_WAIT_TIME);
    } else {
        sleep(WAIT_TIME);
    }
}

test!(multi_with_eol, |dir: WorkingDir, mut cmd: Command| {
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.put_file("file1", "test1!\n");
    sleep(WAIT_TIME);
    dir.put_file("file2", "test2!\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file1");
    assert_contains!(output, "file2");
    assert_contains!(output, " <==\ntest1!\n\n==>");
    assert_contains!(output, " <==\ntest2!\n");
});

test!(multi_without_eol, |dir: WorkingDir, mut cmd: Command| {
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.put_file("file1", "test1!");
    sleep(WAIT_TIME);
    dir.put_file("file2", "test2!");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file1");
    assert_contains!(output, "file2");
    assert_contains!(output, " <==\ntest1!\n\n==>");
    assert_contains!(output, " <==\ntest2!");
});

test!(multi_alread_exist, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file1", "test1!\n");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("file1", "test2!\n");
    sleep(WAIT_TIME);
    dir.put_file("file2", "test3!\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file1");
    assert_contains!(output, "file2");
    assert_contains!(output, "file1 <==\ntest1!\ntest2!\n");
    assert_contains!(output, "file2 <==\ntest3!\n");
});

test!(rename, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file1", "test1");
    sleep(RENAME_WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.rename_file("file1", "file2");
    sleep_for_rename();
    dir.append_file("file2", "test2");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file1 <==\ntest1\n\n==>");
    assert_contains!(output, "file2 <==\ntest2");
});

test!(rename_back, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file1", "test1");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.rename_file("file1", "file2");

    sleep_for_rename();
    dir.append_file("file2", "test2");
    sleep_for_rename();
    dir.rename_file("file2", "file1");
    sleep_for_rename();
    dir.append_file("file1", "test3");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file1 <==\ntest1\n\n==>");
    assert_contains!(output, "file2 <==\ntest2\n\n==>");
    assert_contains!(output, "file1 <==\ntest3");
});
