// Copyright (C) 2024 Bellande Architecture Mechanism Research Innovation Center, Ronaldson Bellande

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::interpreter::interpreter::Interpreter;
use crate::utilities::utilities::{ASTNode, RedirectType};
use glob::glob;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Cursor, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Processes {
    background_jobs: Arc<Mutex<Vec<Arc<Mutex<Child>>>>>,
}

impl Processes {
    pub fn new() -> Self {
        Processes {
            background_jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn execute_command(
        &self,
        interpreter: &mut Interpreter,
        name: String,
        args: Vec<String>,
    ) -> Result<Option<i32>, String> {
        let expanded_name = interpreter.expand_variables(&name);
        let expanded_args: Vec<String> = args
            .iter()
            .map(|arg| interpreter.expand_variables(arg))
            .collect();

        match expanded_name.as_str() {
            "echo" => {
                println!("{}", expanded_args.join(" "));
                Ok(Some(0))
            }
            "cd" => {
                let path = if expanded_args.is_empty() {
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                } else {
                    expanded_args[0].clone()
                };
                if let Err(e) = std::env::set_current_dir(&path) {
                    Err(format!("cd: {}", e))
                } else {
                    Ok(Some(0))
                }
            }
            "exit" => std::process::exit(0),
            "export" => {
                for arg in expanded_args {
                    let parts: Vec<&str> = arg.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        std::env::set_var(parts[0], parts[1]);
                    }
                }
                Ok(Some(0))
            }
            "jobs" => {
                let jobs = self.background_jobs.lock().unwrap();
                for (i, _) in jobs.iter().enumerate() {
                    println!("[{}] Running", i + 1);
                }
                Ok(Some(0))
            }
            _ => {
                if let Some(func) = interpreter.functions.get(&expanded_name) {
                    return interpreter.interpret_node(Box::new(func.clone()));
                }
                match Command::new(&expanded_name).args(&expanded_args).spawn() {
                    Ok(mut child) => {
                        let status = child.wait().map_err(|e| e.to_string())?;
                        Ok(Some(status.code().unwrap_or(0)))
                    }
                    Err(e) => Err(format!("Failed to execute command: {}", e)),
                }
            }
        }
    }

    pub fn execute_pipeline(
        &self,
        interpreter: &Interpreter,
        commands: Vec<ASTNode>,
    ) -> Result<Option<i32>, String> {
        let mut previous_stdout = None;
        let mut processes = Vec::new();

        for (i, command) in commands.iter().enumerate() {
            match command {
                ASTNode::Command { name, args } => {
                    let mut cmd = Command::new(interpreter.expand_variables(name));
                    for arg in args {
                        cmd.arg(interpreter.expand_variables(arg));
                    }

                    if let Some(prev_stdout) = previous_stdout.take() {
                        cmd.stdin(prev_stdout);
                    }

                    if i < commands.len() - 1 {
                        cmd.stdout(Stdio::piped());
                    }

                    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
                    if i < commands.len() - 1 {
                        previous_stdout = child.stdout.take();
                    }
                    processes.push(child);
                }
                _ => return Err("Pipeline can only contain commands".to_string()),
            }
        }

        let mut last_status = None;
        for mut process in processes {
            let status = process.wait().map_err(|e| e.to_string())?;
            last_status = Some(status.code().unwrap_or(0));
        }

        Ok(last_status)
    }

    pub fn execute_redirect(
        &self,
        interpreter: &mut Interpreter,
        node: ASTNode,
        direction: RedirectType,
        target: String,
    ) -> Result<Option<i32>, String> {
        let target = interpreter.expand_variables(&target);
        match direction {
            RedirectType::Out => {
                let file = File::create(&target).map_err(|e| e.to_string())?;
                let mut writer = BufWriter::new(file);
                let result = self.capture_output(interpreter, Box::new(node))?;
                writer
                    .write_all(result.as_bytes())
                    .map_err(|e| e.to_string())?;
                Ok(Some(0))
            }
            RedirectType::Append => {
                let file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(&target)
                    .map_err(|e| e.to_string())?;
                let mut writer = BufWriter::new(file);
                let result = self.capture_output(interpreter, Box::new(node))?;
                writer
                    .write_all(result.as_bytes())
                    .map_err(|e| e.to_string())?;
                Ok(Some(0))
            }
            RedirectType::In => {
                let file = File::open(&target).map_err(|e| e.to_string())?;
                let mut reader = BufReader::new(file);
                let mut input = String::new();
                reader
                    .read_to_string(&mut input)
                    .map_err(|e| e.to_string())?;
                self.execute_with_input(interpreter, Box::new(node), input)
            }
        }
    }

    fn capture_output(
        &self,
        interpreter: &mut Interpreter,
        node: Box<ASTNode>,
    ) -> Result<String, String> {
        let old_stdout = io::stdout();
        let mut handle = old_stdout.lock();
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            let result = interpreter.interpret_node(node)?;
            writeln!(cursor, "{:?}", result).map_err(|e| e.to_string())?;
        }
        handle.write_all(&buffer).map_err(|e| e.to_string())?;
        String::from_utf8(buffer).map_err(|e| e.to_string())
    }

    fn execute_with_input(
        &self,
        interpreter: &mut Interpreter,
        node: Box<ASTNode>,
        input: String,
    ) -> Result<Option<i32>, String> {
        std::env::set_var("BELLOS_INPUT", input);
        interpreter.interpret_node(node)
    }

    pub fn execute_background(&self, node: ASTNode) -> Result<Option<i32>, String> {
        let bg_jobs = Arc::clone(&self.background_jobs);

        // Create a new background process
        let child = Arc::new(Mutex::new(
            Command::new(std::env::current_exe().expect("Failed to get current executable path"))
                .arg("--execute-bellos-script")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to spawn background process: {}", e))?,
        ));

        // Add the new job to the list
        bg_jobs.lock().unwrap().push(Arc::clone(&child));

        thread::spawn(move || {
            let mut interpreter = Interpreter::new();
            if let Err(e) = interpreter.interpret_node(Box::new(node)) {
                eprintln!("Background job error: {}", e);
            }

            let mut jobs = bg_jobs.lock().unwrap();
            jobs.retain(|job| {
                let mut child = job.lock().unwrap();
                match child.try_wait() {
                    Ok(Some(_)) => {
                        println!("Job completed.");
                        false // Job has completed, remove it
                    }
                    Ok(None) => {
                        println!("Job still running.");
                        true // Job is still running, keep it
                    }
                    Err(err) => {
                        eprintln!("Error waiting for job: {}", err);
                        false // Error occurred, remove the job
                    }
                }
            });
        });

        Ok(None)
    }

    pub fn expand_wildcards(&self, pattern: &str) -> Vec<String> {
        match glob(pattern) {
            Ok(paths) => paths
                .filter_map(Result::ok)
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            Err(_) => vec![pattern.to_string()],
        }
    }
}
