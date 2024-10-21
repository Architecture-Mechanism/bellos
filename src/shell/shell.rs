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
use crate::lexer::lexer::Lexer;
use crate::parser::parser::Parser;
use crate::utilities::utilities::{ASTNode, RedirectType};
use std::io::{self, Write};
use std::process::{Command, Stdio};

pub struct Shell {
    pub interpreter: Interpreter,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            interpreter: Interpreter::new(),
        }
    }

    pub fn run(&mut self, input: &str) -> Result<(), String> {
        let mut lexer = Lexer::new(input.to_string());
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        self.interpret(ast)
    }

    pub fn interpret(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            if let Err(e) = self.interpret_node(&node) {
                eprintln!("Error executing command: {}", e);
            }
        }
        Ok(())
    }

    pub fn interpret_node(&mut self, node: &ASTNode) -> Result<Option<i32>, String> {
        match node {
            ASTNode::Command { name, args } => self.execute_command(name, args),
            ASTNode::Pipeline(commands) => self.execute_pipeline(commands),
            ASTNode::Redirect {
                node,
                direction,
                target,
            } => self.execute_redirect(node, direction, target),
            ASTNode::Background(node) => self.execute_background(node),
            _ => self.interpreter.interpret_node(node),
        }
    }

    pub fn execute_command(&mut self, name: &str, args: &[String]) -> Result<Option<i32>, String> {
        let expanded_name = self.interpreter.expand_variables(name);
        let expanded_args: Vec<String> = args
            .iter()
            .map(|arg| self.interpreter.expand_variables(arg))
            .collect();

        let output = Command::new(&expanded_name)
            .args(&expanded_args)
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        io::stdout()
            .write_all(&output.stdout)
            .map_err(|e| e.to_string())?;
        io::stderr()
            .write_all(&output.stderr)
            .map_err(|e| e.to_string())?;

        Ok(Some(output.status.code().unwrap_or(-1)))
    }

    pub fn execute_pipeline(&mut self, commands: &[ASTNode]) -> Result<Option<i32>, String> {
        let mut last_output = Vec::new();
        let mut last_exit_code = None;

        for (i, command) in commands.iter().enumerate() {
            if let ASTNode::Command { name, args } = command {
                let expanded_name = self.interpreter.expand_variables(name);
                let expanded_args: Vec<String> = args
                    .iter()
                    .map(|arg| self.interpreter.expand_variables(arg))
                    .collect();

                let mut process = Command::new(&expanded_name);
                process.args(&expanded_args);

                if i == 0 {
                    process.stdin(Stdio::inherit());
                } else {
                    process.stdin(Stdio::piped());
                }

                if i == commands.len() - 1 {
                    process.stdout(Stdio::inherit());
                } else {
                    process.stdout(Stdio::piped());
                }

                let mut child = process
                    .spawn()
                    .map_err(|e| format!("Failed to spawn process: {}", e))?;

                if i > 0 {
                    if let Some(mut stdin) = child.stdin.take() {
                        stdin
                            .write_all(&last_output)
                            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
                    }
                }

                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to wait for process: {}", e))?;

                last_output = output.stdout;
                last_exit_code = Some(output.status.code().unwrap_or(-1));
            } else {
                return Err("Invalid command in pipeline".to_string());
            }
        }

        Ok(last_exit_code)
    }

    pub fn execute_redirect(
        &mut self,
        node: &ASTNode,
        direction: &RedirectType,
        target: &str,
    ) -> Result<Option<i32>, String> {
        let expanded_target = self.interpreter.expand_variables(target);
        match direction {
            RedirectType::Input => self.execute_input_redirect(node, &expanded_target),
            RedirectType::Output => self.execute_output_redirect(node, &expanded_target),
            RedirectType::Append => self.execute_append_redirect(node, &expanded_target),
        }
    }

    fn execute_input_redirect(
        &mut self,
        node: &ASTNode,
        target: &str,
    ) -> Result<Option<i32>, String> {
        if let ASTNode::Command { name, args } = node {
            let expanded_name = self.interpreter.expand_variables(name);
            let expanded_args: Vec<String> = args
                .iter()
                .map(|arg| self.interpreter.expand_variables(arg))
                .collect();

            let input = std::fs::File::open(target)
                .map_err(|e| format!("Failed to open input file '{}': {}", target, e))?;

            let output = std::process::Command::new(&expanded_name)
                .args(&expanded_args)
                .stdin(input)
                .output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            io::stdout()
                .write_all(&output.stdout)
                .map_err(|e| e.to_string())?;
            io::stderr()
                .write_all(&output.stderr)
                .map_err(|e| e.to_string())?;

            Ok(Some(output.status.code().unwrap_or(-1)))
        } else {
            Err("Invalid command for input redirection".to_string())
        }
    }

    fn execute_output_redirect(
        &mut self,
        node: &ASTNode,
        target: &str,
    ) -> Result<Option<i32>, String> {
        if let ASTNode::Command { name, args } = node {
            let expanded_name = self.interpreter.expand_variables(name);
            let expanded_args: Vec<String> = args
                .iter()
                .map(|arg| self.interpreter.expand_variables(arg))
                .collect();

            let output_file = std::fs::File::create(target)
                .map_err(|e| format!("Failed to create output file '{}': {}", target, e))?;

            let output = std::process::Command::new(&expanded_name)
                .args(&expanded_args)
                .stdout(output_file)
                .output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            io::stderr()
                .write_all(&output.stderr)
                .map_err(|e| e.to_string())?;

            Ok(Some(output.status.code().unwrap_or(-1)))
        } else {
            Err("Invalid command for output redirection".to_string())
        }
    }

    fn execute_append_redirect(
        &mut self,
        node: &ASTNode,
        target: &str,
    ) -> Result<Option<i32>, String> {
        if let ASTNode::Command { name, args } = node {
            let expanded_name = self.interpreter.expand_variables(name);
            let expanded_args: Vec<String> = args
                .iter()
                .map(|arg| self.interpreter.expand_variables(arg))
                .collect();

            let output_file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(target)
                .map_err(|e| format!("Failed to open file '{}' for appending: {}", target, e))?;

            let output = std::process::Command::new(&expanded_name)
                .args(&expanded_args)
                .stdout(output_file)
                .output()
                .map_err(|e| format!("Failed to execute command: {}", e))?;

            io::stderr()
                .write_all(&output.stderr)
                .map_err(|e| e.to_string())?;

            Ok(Some(output.status.code().unwrap_or(-1)))
        } else {
            Err("Invalid command for append redirection".to_string())
        }
    }

    pub fn execute_background(&mut self, node: &ASTNode) -> Result<Option<i32>, String> {
        if let ASTNode::Command { name, args } = node {
            let expanded_name = self.interpreter.expand_variables(name);
            let expanded_args: Vec<String> = args
                .iter()
                .map(|arg| self.interpreter.expand_variables(arg))
                .collect();

            let child = Command::new(&expanded_name)
                .args(&expanded_args)
                .spawn()
                .map_err(|e| format!("Failed to spawn background process: {}", e))?;

            println!("Started background process with PID: {}", child.id());
            Ok(Some(0))
        } else {
            Err("Invalid command for background execution".to_string())
        }
    }
}
