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

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub fn setup(test_directory: &str) -> (WorkingDir, Command) {
    let dir = PathBuf::from(format!("integration_tests/{}", test_directory));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let test_exec_path = std::env::current_exe().unwrap();
    let exec_dir = test_exec_path.parent().unwrap().parent().unwrap();
    let mut exec_path = exec_dir.to_path_buf();
    exec_path.push("regtail");
    let mut command = Command::new(dbg!(exec_path));
    command.stdout(Stdio::piped());
    let working_dir = WorkingDir::create(dir);
    (working_dir, command)
}

pub struct WorkingDir {
    parent_path: PathBuf,
}

impl WorkingDir {
    pub fn create(working_directory: PathBuf) -> Self {
        WorkingDir {
            parent_path: working_directory
        }
    }

    pub fn put_file(self: &Self, relative_path: &str, content: &str) {
        let mut new_file_path = self.parent_path.clone();
        new_file_path.push(relative_path);
        if let Some(parent_dir) = new_file_path.parent() {
            let _ = fs::create_dir_all(parent_dir);
        }
        fs::write(new_file_path, content).expect("Cannot put file");
    }

    pub fn append_file(self: &Self, relative_path: &str, content: &str) {
        let mut append_file_path = self.parent_path.clone();
        append_file_path.push(relative_path);
        let file_path_str = append_file_path.display().to_string();
        let mut fh = OpenOptions::new().append(true).open(append_file_path)
            .expect(format!("Failed to open '{}' with append mode", file_path_str).as_ref());
        let _ = fh.write_all(content.as_bytes());
    }

    pub fn remove_file(self: &Self, relative_path: &str) {
        let mut remove_file_path = self.parent_path.clone();
        remove_file_path.push(relative_path);
        fs::remove_file(remove_file_path).expect("Cannot remove file");
    }

    pub fn rename_file(self: &Self, src_relative_path: &str, dest_relative_path: &str) {
        let mut src_file_path = self.parent_path.clone();
        src_file_path.push(src_relative_path);
        let mut dest_file_path = self.parent_path.clone();
        dest_file_path.push(dest_relative_path);
        fs::rename(src_file_path, dest_file_path).expect("Cannot rename file");
    }

    pub fn display(self: &Self) -> std::path::Display {
        self.parent_path.display()
    }

    pub fn path_arg(self: &Self) -> String {
        format!("-p={}", self.display())
    }
}

pub struct RunningCommand {
    child: Child,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum KillStatus {
    AlreadyExited,
    Killed,
}

impl RunningCommand {
    pub fn create(child: Child) -> Self {
        RunningCommand {
            child
        }
    }

    pub fn exit(self: &mut Self) -> KillStatus {
        let kill_result = self.child.kill().err().map_or(KillStatus::Killed, |_| {
            KillStatus::AlreadyExited
        });
        self.child.wait().unwrap();
        kill_result
    }

    pub fn output(self: &mut Self) -> String {
        let mut output = String::new();
        let _size = self.child.stdout.as_mut().unwrap().read_to_string(&mut output);
        output
    }
}