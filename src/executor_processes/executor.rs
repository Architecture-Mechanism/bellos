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

use crate::executor_processes::processes::Processes;
use crate::interpreter::interpreter::Interpreter;
use crate::lexer::lexer::Lexer;
use crate::parser::parser::Parser;
use crate::utilities::utilities::ASTNode;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Arc;

pub struct Executor {
    interpreter: Interpreter,
    processes: Arc<Processes>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            interpreter: Interpreter::new(),
            processes: Arc::new(Processes::new()),
        }
    }

    pub fn run(&mut self, args: Vec<String>) -> Result<(), String> {
        if args.len() > 1 {
            // Execute script file
            self.execute_script(&args[1])
        } else {
            // Interactive mode
            self.run_interactive_mode()
        }
    }

    fn execute_script(&mut self, filename: &str) -> Result<(), String> {
        println!("Executing script: {}", filename);

        if !filename.ends_with(".bellos") {
            return Err(format!("Not a .bellos script: {}", filename));
        }

        let path = Path::new(filename);
        if !path.exists() {
            return Err(format!("Script file does not exist: {}", filename));
        }

        let file =
            File::open(path).map_err(|e| format!("Error opening file {}: {}", filename, e))?;
        let reader = BufReader::new(file);

        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Error reading line {}: {}", index + 1, e))?;
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                continue; // Skip empty lines and comments
            }

            // Handle variable assignments and arithmetic operations
            if trimmed_line.contains('=') {
                let parts: Vec<&str> = trimmed_line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let var_name = parts[0].trim().to_string();
                    let var_value = parts[1].trim().to_string();

                    if var_value.starts_with("$((") && var_value.ends_with("))") {
                        // Arithmetic expression
                        let result = self.interpreter.evaluate_arithmetic(&var_value)?;
                        self.interpreter
                            .variables
                            .insert(var_name, result.to_string());
                    } else {
                        // Regular variable assignment
                        let expanded_value = self.interpreter.expand_variables(&var_value);
                        self.interpreter.variables.insert(var_name, expanded_value);
                    }
                    continue;
                }
            }

            if let Err(e) = self.process_content(trimmed_line) {
                return Err(format!("Error on line {}: {}", index + 1, e));
            }
        }
        Ok(())
    }

    fn run_interactive_mode(&mut self) -> Result<(), String> {
        loop {
            print!("bellos> ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            if input.trim().is_empty() {
                continue;
            }

            if let Err(e) = self.process_content(&input) {
                eprintln!("Error: {}", e);
            }
        }
    }

    fn process_content(&mut self, content: &str) -> Result<(), String> {
        // Handle variable assignments and arithmetic operations
        if content.contains('=') {
            let parts: Vec<&str> = content.splitn(2, '=').collect();
            if parts.len() == 2 {
                let var_name = parts[0].trim().to_string();
                let var_value = parts[1].trim().to_string();

                if var_value.starts_with("$((") && var_value.ends_with("))") {
                    // Arithmetic expression
                    let result = self.interpreter.evaluate_arithmetic(&var_value)?;
                    self.interpreter
                        .variables
                        .insert(var_name, result.to_string());
                } else {
                    // Regular variable assignment
                    let expanded_value = self.interpreter.expand_variables(&var_value);
                    self.interpreter.variables.insert(var_name, expanded_value);
                }
                return Ok(());
            }
        }

        let ast_nodes = self.parse_content(content)?;
        self.execute(ast_nodes)
    }

    fn parse_content(&self, content: &str) -> Result<Vec<ASTNode>, String> {
        let mut lexer = Lexer::new(content.to_string());
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    pub fn execute(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            self.execute_node(node)?;
        }
        Ok(())
    }

    fn execute_node(&mut self, node: ASTNode) -> Result<Option<i32>, String> {
        match node {
            ASTNode::Command { name, args } => Arc::get_mut(&mut self.processes)
                .unwrap()
                .execute_command(&mut self.interpreter, name, args),
            ASTNode::Pipeline(commands) => {
                self.processes.execute_pipeline(&self.interpreter, commands)
            }
            ASTNode::Redirect {
                node,
                direction,
                target,
            } => Arc::get_mut(&mut self.processes).unwrap().execute_redirect(
                &mut self.interpreter,
                *node,
                direction,
                target,
            ),
            ASTNode::Background(node) => self.processes.execute_background(*node),
            _ => self.interpreter.interpret_node(Box::new(node)),
        }
    }
}
