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

test!(simple_run, |dir: WorkingDir, mut cmd: Command| {
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.put_file("writed", "tests!");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "tests!");
});

test!(append_content, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("appended", "line1\nline2\nline3");
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("appended", "\nline4\nline5\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "line1\nline2\nline3\nline4\nline5\n");
});

test!(rm_append, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("removed_file", "line1\n");
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.remove_file("removed_file");
    sleep(WAIT_TIME);
    dir.put_file("removed_file", "line2\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "line1\n\n==>");
    assert_contains!(output, "removed_file <==\nline2");
});

test!(double_append, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("double", "line1\n");
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("double", "line2");
    sleep(WAIT_TIME);
    dir.append_file("double", "appended\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "double <==\nline1\nline2appended\n");
});

test!(shrink_append, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("replaced", "line1\n");
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.shrink_file("replaced");
    sleep(WAIT_TIME);
    dir.append_file("replaced", "new line1\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "replaced <==\nline1\nnew line1\n");
});