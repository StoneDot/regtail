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

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub fn setup(test_directory: &str) -> (WorkingDir, Command) {
    let dir = PathBuf::from(format!("integration_tests/{}", test_directory));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let mut command = Command::new("target/debug/regtail");
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

impl RunningCommand {
    pub fn create(child: Child) -> Self {
        RunningCommand {
            child
        }
    }

    pub fn exit(self: &mut Self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }

    pub fn output(self: &mut Self) -> String {
        let mut output = String::new();
        let _size = self.child.stdout.as_mut().unwrap().read_to_string(&mut output);
        output
    }
}