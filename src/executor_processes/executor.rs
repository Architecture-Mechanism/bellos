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
            self.execute_script(&args[1])
        } else {
            self.run_interactive_mode()
        }
    }

    fn execute_script(&mut self, filename: &str) -> Result<(), String> {
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
                continue;
            }

            if let Err(e) = self.process_content(trimmed_line) {
                eprintln!("Error on line {}: {}", index + 1, e);
            }
            io::stdout().flush().unwrap();
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
            ASTNode::Assignment { name, value } => {
                let expanded_value = self.interpreter.expand_variables(&value);
                self.interpreter.variables.insert(name, expanded_value);
                Ok(None)
            }
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
