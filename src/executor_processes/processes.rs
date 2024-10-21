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

use crate::interpreter_logic::interpreter::Interpreter;
use crate::interpreter_logic::logic::Logic;
use crate::utilities::utilities::{ASTNode, RedirectType};
use glob::glob;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Cursor, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Processes {
    background_jobs: Arc<Mutex<Vec<Arc<Mutex<Child>>>>>,
    pub logic: Logic,
}

impl Processes {
    pub fn new() -> Self {
        Processes {
            background_jobs: Arc::new(Mutex::new(Vec::new())),
            logic: Logic::new(),
        }
    }

    pub fn execute_command(
        &mut self,
        interpreter: &mut Interpreter,
        name: &str,
        args: &[String],
    ) -> Result<Option<i32>, String> {
        match name {
            "echo" => self.builtin_echo(interpreter, args),
            "exit" => std::process::exit(0),
            "export" => self.builtin_export(interpreter, args),
            "jobs" => self.builtin_jobs(),
            "write" => self.builtin_write(args),
            "read" => self.builtin_read(args),
            "append" => self.builtin_append(args),
            "delete" => self.builtin_delete(args),
            "[" => self.evaluate_condition(interpreter, args),
            "seq" => self.builtin_seq(args),
            _ => self.execute_external_command(name, args),
        }
    }

    fn builtin_echo(
        &self,
        interpreter: &mut Interpreter,
        args: &[String],
    ) -> Result<Option<i32>, String> {
        let expanded_args: Vec<String> = args
            .iter()
            .map(|arg| interpreter.expand_variables(arg))
            .collect();
        println!("{}", expanded_args.join(" "));
        Ok(Some(0))
    }

    fn builtin_export(
        &self,
        interpreter: &mut Interpreter,
        args: &[String],
    ) -> Result<Option<i32>, String> {
        for arg in args {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                std::env::set_var(parts[0], parts[1]);
                interpreter
                    .variables
                    .insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        Ok(Some(0))
    }

    fn builtin_jobs(&self) -> Result<Option<i32>, String> {
        let jobs = self.background_jobs.lock().unwrap();
        for (i, _) in jobs.iter().enumerate() {
            println!("[{}] Running", i + 1);
        }
        Ok(Some(0))
    }

    fn builtin_write(&self, args: &[String]) -> Result<Option<i32>, String> {
        if args.len() != 2 {
            return Err("Usage: write <filename> <content>".to_string());
        }
        let filename = &args[0];
        let content = &args[1];
        let mut file = File::create(filename)
            .map_err(|e| format!("Failed to create file {}: {}", filename, e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write to file {}: {}", filename, e))?;
        Ok(Some(0))
    }

    fn builtin_read(&self, args: &[String]) -> Result<Option<i32>, String> {
        if args.len() != 1 {
            return Err("Usage: read <filename>".to_string());
        }
        let filename = &args[0];
        let mut content = String::new();
        File::open(filename)
            .map_err(|e| format!("Failed to open file {}: {}", filename, e))?
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read file {}: {}", filename, e))?;
        println!("{}", content);
        Ok(Some(0))
    }

    fn builtin_append(&self, args: &[String]) -> Result<Option<i32>, String> {
        if args.len() != 2 {
            return Err("Usage: append <filename> <content>".to_string());
        }
        let filename = &args[0];
        let content = &args[1];
        let mut file = OpenOptions::new()
            .append(true)
            .open(filename)
            .map_err(|e| format!("Failed to open file {}: {}", filename, e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to append to file {}: {}", filename, e))?;
        Ok(Some(0))
    }

    fn builtin_delete(&self, args: &[String]) -> Result<Option<i32>, String> {
        if args.len() != 1 {
            return Err("Usage: delete <filename>".to_string());
        }
        let filename = &args[0];
        std::fs::remove_file(filename)
            .map_err(|e| format!("Failed to delete file {}: {}", filename, e))?;
        Ok(Some(0))
    }

    fn builtin_seq(&self, args: &[String]) -> Result<Option<i32>, String> {
        if args.len() < 1 || args.len() > 3 {
            return Err("Usage: seq [START] [STEP] END".to_string());
        }

        let (start, step, end) = match args.len() {
            1 => (
                1,
                1,
                args[0]
                    .parse::<i32>()
                    .map_err(|_| "Invalid number".to_string())?,
            ),
            2 => (
                args[0]
                    .parse::<i32>()
                    .map_err(|_| "Invalid number".to_string())?,
                1,
                args[1]
                    .parse::<i32>()
                    .map_err(|_| "Invalid number".to_string())?,
            ),
            3 => (
                args[0]
                    .parse::<i32>()
                    .map_err(|_| "Invalid number".to_string())?,
                args[1]
                    .parse::<i32>()
                    .map_err(|_| "Invalid number".to_string())?,
                args[2]
                    .parse::<i32>()
                    .map_err(|_| "Invalid number".to_string())?,
            ),
            _ => unreachable!(),
        };

        for i in (start..=end).step_by(step as usize) {
            println!("{}", i);
        }
        Ok(Some(0))
    }

    fn execute_external_command(&self, name: &str, args: &[String]) -> Result<Option<i32>, String> {
        match Command::new(name).args(args).spawn() {
            Ok(mut child) => {
                let status = child.wait().map_err(|e| e.to_string())?;
                Ok(Some(status.code().unwrap_or(0)))
            }
            Err(e) => Err(format!("Failed to execute command: {}", e)),
        }
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
            RedirectType::Output => self.execute_output_redirect(interpreter, node, &target),
            RedirectType::Append => self.execute_append_redirect(interpreter, node, &target),
            RedirectType::Input => self.execute_input_redirect(interpreter, node, &target),
        }
    }

    fn execute_output_redirect(
        &self,
        interpreter: &mut Interpreter,
        node: ASTNode,
        target: &str,
    ) -> Result<Option<i32>, String> {
        let file = File::create(target).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        let result = self.capture_output(interpreter, Box::new(node))?;
        writer
            .write_all(result.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(Some(0))
    }

    fn execute_append_redirect(
        &self,
        interpreter: &mut Interpreter,
        node: ASTNode,
        target: &str,
    ) -> Result<Option<i32>, String> {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(target)
            .map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        let result = self.capture_output(interpreter, Box::new(node))?;
        writer
            .write_all(result.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(Some(0))
    }

    fn execute_input_redirect(
        &self,
        interpreter: &mut Interpreter,
        node: ASTNode,
        target: &str,
    ) -> Result<Option<i32>, String> {
        let file = File::open(target).map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(file);
        let mut input = String::new();
        reader
            .read_to_string(&mut input)
            .map_err(|e| e.to_string())?;
        self.execute_with_input(interpreter, node, input)
    }

    pub fn execute_pipeline(
        &self,
        interpreter: &mut Interpreter,
        commands: Vec<ASTNode>,
    ) -> Result<Option<i32>, String> {
        let mut previous_stdout = None;
        let mut processes = Vec::new();

        for (i, command) in commands.iter().enumerate() {
            match command {
                ASTNode::Command { name, args } => {
                    let process = self.setup_pipeline_command(
                        interpreter,
                        name,
                        args,
                        i,
                        &commands.len(),
                        &mut previous_stdout,
                    )?;
                    processes.push(process);
                }
                _ => return Err("Pipeline can only contain commands".to_string()),
            }
        }

        self.wait_for_processes(processes)
    }

    fn setup_pipeline_command(
        &self,
        interpreter: &mut Interpreter,
        name: &str,
        args: &[String],
        index: usize,
        total_commands: &usize,
        previous_stdout: &mut Option<Stdio>,
    ) -> Result<Child, String> {
        let mut cmd = Command::new(interpreter.expand_variables(name));
        for arg in args {
            cmd.arg(interpreter.expand_variables(arg));
        }

        if let Some(prev_stdout) = previous_stdout.take() {
            cmd.stdin(prev_stdout);
        }

        if index < total_commands - 1 {
            cmd.stdout(Stdio::piped());
        }

        let mut child = cmd.spawn().map_err(|e| e.to_string())?;

        if index < total_commands - 1 {
            *previous_stdout = child.stdout.take().map(Stdio::from);
        }

        Ok(child)
    }

    fn wait_for_processes(&self, processes: Vec<Child>) -> Result<Option<i32>, String> {
        let mut last_status = None;
        for mut process in processes {
            let status = process.wait().map_err(|e| e.to_string())?;
            last_status = Some(status.code().unwrap_or(0));
        }
        Ok(last_status)
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
            let result = interpreter.interpret_node(&node)?;
            writeln!(cursor, "{:?}", result).map_err(|e| e.to_string())?;
        }
        handle.write_all(&buffer).map_err(|e| e.to_string())?;
        String::from_utf8(buffer).map_err(|e| e.to_string())
    }

    fn execute_with_input(
        &self,
        interpreter: &mut Interpreter,
        node: ASTNode,
        input: String,
    ) -> Result<Option<i32>, String> {
        std::env::set_var("BELLOS_INPUT", input);
        interpreter.interpret_node(&node)
    }

    pub fn execute_background(
        &mut self,
        interpreter: &mut Interpreter,
        node: ASTNode,
    ) -> Result<Option<i32>, String> {
        let bg_jobs = Arc::clone(&self.background_jobs);
        let interpreter_clone = interpreter.clone();

        thread::spawn(move || {
            let mut local_interpreter = interpreter_clone;
            if let Err(e) = local_interpreter.interpret_node(&node) {
                eprintln!("Background job error: {}", e);
            }

            let mut jobs = bg_jobs.lock().unwrap();
            jobs.retain(|job| {
                let mut child = job.lock().unwrap();
                match child.try_wait() {
                    Ok(Some(_)) => {
                        println!("Job completed.");
                        false
                    }
                    Ok(None) => {
                        println!("Job still running.");
                        true
                    }
                    Err(err) => {
                        eprintln!("Error waiting for job: {}", err);
                        false
                    }
                }
            });
        });

        let placeholder =
            Arc::new(Mutex::new(Command::new("sleep").arg("1").spawn().map_err(
                |e| format!("Failed to create placeholder process: {}", e),
            )?));
        self.background_jobs.lock().unwrap().push(placeholder);

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

    fn evaluate_condition(
        &self,
        interpreter: &mut Interpreter,
        args: &[String],
    ) -> Result<Option<i32>, String> {
        if args.len() != 3 {
            return Err("Invalid condition syntax".to_string());
        }
        let result =
            self.logic
                .compare_values(&interpreter.variables, &args[0], &args[1], &args[2])?;
        Ok(Some(if result { 0 } else { 1 }))
    }
}
