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

use crate::utilities::utilities::{ASTNode, RedirectType};
use glob::glob;
use std::collections::HashMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Interpreter {
    variables: HashMap<String, String>,
    functions: HashMap<String, ASTNode>,
    background_jobs: Arc<Mutex<Vec<Child>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            functions: HashMap::new(),
            background_jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn interpret(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            self.interpret_node(Box::new(node))?;
        }
        Ok(())
    }

    fn interpret_node(&mut self, node: Box<ASTNode>) -> Result<Option<i32>, String> {
        match *node {
            ASTNode::Command { name, args } => self.execute_command(name, args),
            ASTNode::Assignment { name, value } => {
                let expanded_value = self.expand_variables(&value);
                self.variables.insert(name, expanded_value);
                Ok(None)
            }
            ASTNode::Pipeline(commands) => self.execute_pipeline(commands),
            ASTNode::Redirect {
                node,
                direction,
                target,
            } => self.execute_redirect(*node, direction, target),
            ASTNode::Block(statements) => {
                for statement in statements {
                    self.interpret_node(Box::new(statement))?;
                }
                Ok(None)
            }
            ASTNode::If {
                condition,
                then_block,
                else_block,
            } => {
                if self.evaluate_condition(&condition)? {
                    self.interpret_node(then_block)?;
                } else if let Some(else_block) = else_block {
                    self.interpret_node(else_block)?;
                }
                Ok(None)
            }
            ASTNode::While { condition, block } => {
                while self.evaluate_condition(&condition)? {
                    self.interpret_node(Box::new(*block.clone()))?;
                }
                Ok(None)
            }
            ASTNode::For { var, list, block } => {
                for item in list {
                    self.variables.insert(var.clone(), item);
                    self.interpret_node(Box::new(*block.clone()))?;
                }
                Ok(None)
            }
            ASTNode::Function { name, body } => {
                self.functions.insert(name, *body);
                Ok(None)
            }
            ASTNode::Background(node) => {
                let bg_jobs = Arc::clone(&self.background_jobs);
                thread::spawn(move || {
                    let mut interpreter = Interpreter::new();
                    interpreter.background_jobs = bg_jobs;
                    if let Err(e) = interpreter.interpret_node(node) {
                        eprintln!("Background job error: {}", e);
                    }
                });
                Ok(None)
            }
        }
    }

    fn execute_command(&mut self, name: String, args: Vec<String>) -> Result<Option<i32>, String> {
        let expanded_name = self.expand_variables(&name);
        let expanded_args: Vec<String> =
            args.iter().map(|arg| self.expand_variables(arg)).collect();

        match expanded_name.as_str() {
            "echo" => {
                println!("{}", expanded_args.join(" "));
                Ok(Some(0))
            }
            "cd" => {
                let path = if expanded_args.is_empty() {
                    env::var("HOME").unwrap_or_else(|_| ".".to_string())
                } else {
                    expanded_args[0].clone()
                };
                if let Err(e) = env::set_current_dir(&path) {
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
                        env::set_var(parts[0], parts[1]);
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
                if let Some(func) = self.functions.get(&expanded_name) {
                    return self.interpret_node(Box::new(func.clone()));
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

    fn evaluate_arithmetic(&self, expr: &str) -> Result<i32, String> {
        let tokens: Vec<&str> = expr.split_whitespace().collect();
        if tokens.len() != 3 {
            return Err("Invalid arithmetic expression".to_string());
        }

        let a = self.get_var_value(tokens[0])?;
        let b = self.get_var_value(tokens[2])?;

        match tokens[1] {
            "+" => Ok(a + b),
            "-" => Ok(a - b),
            "*" => Ok(a * b),
            "/" => {
                if b != 0 {
                    Ok(a / b)
                } else {
                    Err("Division by zero".to_string())
                }
            }
            "%" => {
                if b != 0 {
                    Ok(a % b)
                } else {
                    Err("Modulo by zero".to_string())
                }
            }
            _ => Err(format!("Unsupported operation: {}", tokens[1])),
        }
    }

    fn get_var_value(&self, var: &str) -> Result<i32, String> {
        if let Some(value) = self.variables.get(var) {
            value
                .parse()
                .map_err(|_| format!("Invalid integer: {}", value))
        } else if let Ok(value) = env::var(var) {
            value
                .parse()
                .map_err(|_| format!("Invalid integer: {}", value))
        } else {
            var.parse()
                .map_err(|_| format!("Invalid integer or undefined variable: {}", var))
        }
    }

    fn evaluate_condition(&mut self, condition: &ASTNode) -> Result<bool, String> {
        match condition {
            ASTNode::Command { name, args } => {
                let expanded_args: Vec<String> =
                    args.iter().map(|arg| self.expand_variables(arg)).collect();
                match name.as_str() {
                    "[" | "test" => {
                        if expanded_args.len() < 3 || expanded_args.last() != Some(&"]".to_string())
                        {
                            return Err("Invalid test condition".to_string());
                        }
                        match expanded_args[1].as_str() {
                            "-eq" => Ok(expanded_args[0] == expanded_args[2]),
                            "-ne" => Ok(expanded_args[0] != expanded_args[2]),
                            "-lt" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                < expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-le" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                <= expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-gt" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                > expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-ge" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                >= expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-z" => Ok(expanded_args[0].is_empty()),
                            "-n" => Ok(!expanded_args[0].is_empty()),
                            _ => Err(format!("Unsupported test condition: {}", expanded_args[1])),
                        }
                    }
                    _ => {
                        let result = self.execute_command(name.clone(), expanded_args)?;
                        Ok(result == Some(0))
                    }
                }
            }
            _ => Err("Invalid condition node".to_string()),
        }
    }

    fn execute_pipeline(&mut self, commands: Vec<ASTNode>) -> Result<Option<i32>, String> {
        let mut previous_stdout = None;
        let mut processes = Vec::new();

        for (i, command) in commands.iter().enumerate() {
            match command {
                ASTNode::Command { name, args } => {
                    let mut cmd = Command::new(self.expand_variables(name));
                    for arg in args {
                        cmd.arg(self.expand_variables(arg));
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

    fn execute_redirect(
        &mut self,
        node: ASTNode,
        direction: RedirectType,
        target: String,
    ) -> Result<Option<i32>, String> {
        let target = self.expand_variables(&target);
        match direction {
            RedirectType::Out => {
                let mut file = File::create(&target).map_err(|e| e.to_string())?;
                let result = self.capture_output(Box::new(node))?;
                file.write_all(result.as_bytes())
                    .map_err(|e| e.to_string())?;
                Ok(Some(0))
            }
            RedirectType::Append => {
                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(&target)
                    .map_err(|e| e.to_string())?;
                let result = self.capture_output(Box::new(node))?;
                file.write_all(result.as_bytes())
                    .map_err(|e| e.to_string())?;
                Ok(Some(0))
            }
            RedirectType::In => {
                let mut file = File::open(&target).map_err(|e| e.to_string())?;
                let mut input = String::new();
                file.read_to_string(&mut input).map_err(|e| e.to_string())?;
                self.execute_with_input(Box::new(node), input)
            }
        }
    }

    fn capture_output(&mut self, node: Box<ASTNode>) -> Result<String, String> {
        let old_stdout = io::stdout();
        let mut handle = old_stdout.lock();
        let mut buffer = Vec::new();
        {
            let mut cursor = io::Cursor::new(&mut buffer);
            let result = self.interpret_node(node)?;
            write!(cursor, "{:?}", result).map_err(|e| e.to_string())?;
        }
        handle.write_all(&buffer).map_err(|e| e.to_string())?;
        String::from_utf8(buffer).map_err(|e| e.to_string())
    }

    fn execute_with_input(
        &mut self,
        node: Box<ASTNode>,
        input: String,
    ) -> Result<Option<i32>, String> {
        let mut temp_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        temp_file
            .write_all(input.as_bytes())
            .map_err(|e| e.to_string())?;
        temp_file.flush().map_err(|e| e.to_string())?;

        let input_file = File::open(temp_file.path()).map_err(|e| e.to_string())?;

        let stdin = io::stdin();
        let old_stdin = stdin.lock();
        let new_stdin = unsafe {
            use std::os::unix::io::FromRawFd;
            std::fs::File::from_raw_fd(input_file.as_raw_fd())
        };

        let result = self.interpret_node(node);

        drop(new_stdin);
        drop(old_stdin);

        result
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'(') {
                    chars.next(); // consume '('
                    let expr: String = chars.by_ref().take_while(|&c| c != ')').collect();
                    if expr.starts_with('(') && expr.ends_with(')') {
                        // Arithmetic expression
                        let arithmetic_expr = &expr[1..expr.len() - 1];
                        match self.evaluate_arithmetic(arithmetic_expr) {
                            Ok(value) => result.push_str(&value.to_string()),
                            Err(e) => result.push_str(&format!("Error: {}", e)),
                        }
                    }
                } else {
                    let var_name: String = chars
                        .by_ref()
                        .take_while(|&c| c.is_alphanumeric() || c == '_')
                        .collect();
                    if let Some(value) = self.variables.get(&var_name) {
                        result.push_str(value);
                    } else if let Ok(value) = env::var(&var_name) {
                        result.push_str(&value);
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    fn expand_wildcards(&self, pattern: &str) -> Vec<String> {
        match glob(pattern) {
            Ok(paths) => paths
                .filter_map(Result::ok)
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            Err(_) => vec![pattern.to_string()],
        }
    }
}
