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

const WAIT_TIME: Duration = Duration::from_millis(400);

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
    sleep(WAIT_TIME);
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
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.remove_file("removed_file");
    sleep(WAIT_TIME);

    // On windows, some time required to remove old file because of pending delete
    if cfg!(target_os = "windows") {
        sleep(Duration::from_secs(2));
    }

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
    sleep(WAIT_TIME);
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
    sleep(WAIT_TIME);
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

test!(no_tailing, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file", "should not shown\n");
    sleep(WAIT_TIME);
    let mut child =
        RunningCommand::create(cmd.arg("-l").arg("0").arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("file", "should shown\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file <==\nshould shown\n");
    assert_not_contains!(output, "file <==\nshould not shown\n");
});

test!(multi_byte_file_name, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("日本語ファイル.txt", "表示できます。\n");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("日本語ファイル.txt", "追記します。\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(
        output,
        "日本語ファイル.txt <==\n表示できます。\n追記します。\n"
    );
});

test!(binary_zero_file, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("binfile", "This is binary\0yeah!");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_not_contains!(output, "binfile");
    assert_not_contains!(output, "This is binary");
    assert_not_contains!(output, "yeah!");
});

test!(binary_non_zero_file, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("binfile", b"This is\xa0binary\x88yeah!");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_not_contains!(output, "binfile");
    assert_not_contains!(output, "This is");
    assert_not_contains!(output, "binary");
    assert_not_contains!(output, "yeah!");
});

test!(show_binary_file, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("binfile", "This is not binary\0yeah!");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(
        cmd.arg("--show-binary")
            .arg(dir.path_arg())
            .spawn()
            .unwrap(),
    );
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "binfile");
    assert_contains!(output, "This is not binary");
    assert_contains!(output, "yeah!");
});

test!(show_utf8_bom_file, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("binfile", b"\xef\xbb\xbfThis is not binary\nyeah!");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(
        cmd.arg("--show-binary")
            .arg(dir.path_arg())
            .spawn()
            .unwrap(),
    );
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "binfile");
    assert_contains!(output, "This is not binary");
    assert_contains!(output, "yeah!");
});

test!(filtered, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file", "not shown");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg("none").arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("file", "also not shown");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_not_contains!(output, "file");
    assert_not_contains!(output, "now shown");
    assert_not_contains!(output, "also not shown");
});

test!(no_initial_output, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file", "not shown");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg("-l=0").arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("file", "to be shown");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_not_contains!(output, "now shown");
    assert_contains!(output, "file <==\nto be shown");
    assert_contains!(output, "file <==\nto be shown");
});

#[cfg(target_os = "linux")]
test!(symlink, |dir: WorkingDir, mut cmd: Command| {
    dir.put_file("file", "initial contents\n");
    dir.symlink("file", "link");
    sleep(WAIT_TIME);
    let mut child = RunningCommand::create(cmd.arg("file$").arg(dir.path_arg()).spawn().unwrap());
    sleep(WAIT_TIME);
    dir.append_file("link", "appended\n");
    sleep(WAIT_TIME);
    let result = child.exit();
    assert_eq!(result, KillStatus::Killed);
    let output = child.output();
    assert_contains!(output, "file <==\ninitial contents\nappended");
});
