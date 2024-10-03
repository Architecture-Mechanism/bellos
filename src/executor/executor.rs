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
use crate::lexer::lexer::Lexer;
use crate::parser::parser::Parser;
use crate::utilities::utilities::{ASTNode, RedirectType, Token};
use shellexpand;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};

pub struct Executor {
    interpreter: Interpreter,
    variables: HashMap<String, String>,
    functions: HashMap<String, ASTNode>,
    last_exit_status: i32,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            interpreter: Interpreter::new(),
            variables: HashMap::new(),
            functions: HashMap::new(),
            last_exit_status: 0,
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
        let file =
            File::open(filename).map_err(|e| format!("Error opening file {}: {}", filename, e))?;
        let reader = io::BufReader::new(file);

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Error reading line: {}", e))?;
            self.process_line(&line, line_num + 1)?;
        }
        Ok(())
    }

    fn process_line(&mut self, line: &str, line_num: usize) -> Result<(), String> {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            return Ok(()); // Skip empty lines and comments
        }

        let lexer = Lexer::new(line.to_string());
        let tokens: Vec<Token> = lexer.into_iter().collect();
        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(ast) => {
                if let Err(e) = self.execute(ast) {
                    eprintln!("Error on line {}: {}", line_num, e);
                }
            }
            Err(e) => eprintln!("Parse error on line {}: {}", line_num, e),
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

            let lexer = Lexer::new(input);
            let tokens: Vec<Token> = lexer.into_iter().collect();
            let mut parser = Parser::new(tokens);
            match parser.parse() {
                Ok(ast) => {
                    if let Err(e) = self.execute(ast) {
                        eprintln!("Error: {}", e);
                    }
                }
                Err(e) => eprintln!("Parse error: {}", e),
            }
        }
    }

    pub fn execute(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            self.execute_node(node)?;
        }
        Ok(())
    }

    fn execute_node(&mut self, node: ASTNode) -> Result<String, String> {
        match node {
            ASTNode::Command { name, args } => self.execute_command(name, args),
            ASTNode::Assignment { name, value } => {
                let expanded_value = self.expand_variables(&value);
                self.variables.insert(name, expanded_value);
                Ok(String::new())
            }
            ASTNode::Pipeline(commands) => self.execute_pipeline(commands),
            ASTNode::Redirect {
                node,
                direction,
                target,
            } => self.execute_redirect(*node, direction, target),
            ASTNode::Block(nodes) => {
                let mut last_output = String::new();
                for node in nodes {
                    last_output = self.execute_node(node)?;
                }
                Ok(last_output)
            }
            ASTNode::If {
                condition,
                then_block,
                else_block,
            } => self.execute_if(*condition, *then_block, else_block.map(|b| *b)),
            ASTNode::While { condition, block } => self.execute_while(*condition, *block),
            ASTNode::For { var, list, block } => self.execute_for(var, list, *block),
            ASTNode::Function { name, body } => {
                self.functions.insert(name, *body);
                Ok(String::new())
            }
            ASTNode::Background(node) => self.execute_background(*node),
        }
    }

    fn execute_command(&mut self, name: String, args: Vec<String>) -> Result<String, String> {
        let expanded_args: Vec<String> =
            args.iter().map(|arg| self.expand_variables(arg)).collect();

        let result = match name.as_str() {
            "cd" => self.change_directory(&expanded_args),
            "echo" => {
                let output = expanded_args.join(" ");
                println!("{}", output);
                Ok(output)
            }
            "exit" => std::process::exit(0),
            "write" => self.handle_write(&expanded_args),
            "append" => self.handle_append(&expanded_args),
            "read" => self.handle_read(&expanded_args),
            "read_lines" => self.handle_read_lines(&expanded_args),
            "delete" => self.handle_delete(&expanded_args),
            _ => {
                if let Some(function) = self.functions.get(&name) {
                    self.execute_node(function.clone())
                } else {
                    // Execute external command
                    let output = Command::new(&name)
                        .args(&expanded_args)
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;

                    if output.status.success() {
                        Ok(String::from_utf8_lossy(&output.stdout).to_string())
                    } else {
                        Err(String::from_utf8_lossy(&output.stderr).to_string())
                    }
                }
            }
        };

        self.last_exit_status = if result.is_ok() { 0 } else { 1 };
        result
    }

    fn change_directory(&self, args: &[String]) -> Result<String, String> {
        let new_dir = args.get(0).map(|s| s.as_str()).unwrap_or("~");
        let path = shellexpand::tilde(new_dir);
        std::env::set_current_dir(path.as_ref())
            .map_err(|e| format!("Failed to change directory: {}", e))?;
        Ok(String::new())
    }

    fn execute_pipeline(&mut self, commands: Vec<ASTNode>) -> Result<String, String> {
        let mut last_output = Vec::new();

        for (i, command) in commands.iter().enumerate() {
            let mut child = match command {
                ASTNode::Command { name, args } => {
                    let mut cmd = Command::new(name);
                    cmd.args(args);

                    if i > 0 {
                        cmd.stdin(Stdio::piped());
                    }
                    if i < commands.len() - 1 {
                        cmd.stdout(Stdio::piped());
                    }

                    cmd.spawn()
                        .map_err(|e| format!("Failed to spawn command: {}", e))?
                }
                _ => return Err("Invalid command in pipeline".to_string()),
            };

            if i > 0 {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin
                        .write_all(&last_output)
                        .map_err(|e| format!("Failed to write to stdin: {}", e))?;
                }
            }

            let output = child
                .wait_with_output()
                .map_err(|e| format!("Failed to wait for command: {}", e))?;
            last_output = output.stdout;
        }

        Ok(String::from_utf8_lossy(&last_output).to_string())
    }

    fn execute_redirect(
        &mut self,
        node: ASTNode,
        direction: RedirectType,
        target: String,
    ) -> Result<String, String> {
        let output = self.execute_node(node)?;

        match direction {
            RedirectType::Out => {
                let mut file =
                    File::create(&target).map_err(|e| format!("Failed to create file: {}", e))?;
                file.write_all(output.as_bytes())
                    .map_err(|e| format!("Failed to write to file: {}", e))?;
            }
            RedirectType::Append => {
                let mut file = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&target)
                    .map_err(|e| format!("Failed to open file: {}", e))?;
                file.write_all(output.as_bytes())
                    .map_err(|e| format!("Failed to append to file: {}", e))?;
            }
            RedirectType::In => {
                let mut file =
                    File::open(&target).map_err(|e| format!("Failed to open file: {}", e))?;
                let mut content = String::new();
                file.read_to_string(&mut content)
                    .map_err(|e| format!("Failed to read file: {}", e))?;
                return Ok(content);
            }
        }

        Ok(String::new())
    }

    fn execute_if(
        &mut self,
        condition: ASTNode,
        then_block: ASTNode,
        else_block: Option<ASTNode>,
    ) -> Result<String, String> {
        let condition_result = self.execute_node(condition)?;
        if !condition_result.trim().is_empty() && condition_result.trim() != "0" {
            self.execute_node(then_block)
        } else if let Some(else_block) = else_block {
            self.execute_node(else_block)
        } else {
            Ok(String::new())
        }
    }

    fn execute_while(&mut self, condition: ASTNode, block: ASTNode) -> Result<String, String> {
        let mut last_output = String::new();
        while {
            let condition_result = self.execute_node(condition.clone())?;
            !condition_result.trim().is_empty() && condition_result.trim() != "0"
        } {
            last_output = self.execute_node(block.clone())?;
        }
        Ok(last_output)
    }

    fn execute_for(
        &mut self,
        var: String,
        list: Vec<String>,
        block: ASTNode,
    ) -> Result<String, String> {
        let mut last_output = String::new();
        for item in list {
            self.variables.insert(var.clone(), item);
            last_output = self.execute_node(block.clone())?;
        }
        Ok(last_output)
    }

    fn execute_background(&mut self, node: ASTNode) -> Result<String, String> {
        std::thread::spawn(move || {
            let mut executor = Executor::new();
            if let Err(e) = executor.execute_node(node) {
                eprintln!("Background job error: {}", e);
            }
        });
        Ok(String::new())
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' {
                let var_name: String = chars
                    .by_ref()
                    .take_while(|&c| c.is_alphanumeric() || c == '_')
                    .collect();
                if var_name == "?" {
                    result.push_str(&self.last_exit_status.to_string());
                } else if var_name == "#" {
                    // Assuming we don't have access to script arguments in this context
                    result.push_str("0");
                } else if let Some(value) = self.variables.get(&var_name) {
                    result.push_str(value);
                } else if let Ok(value) = std::env::var(&var_name) {
                    result.push_str(&value);
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    // File handling methods
    fn handle_write(&self, args: &[String]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("Usage: write <filename> <content>".to_string());
        }
        let filename = &args[0];
        let content = &args[1];

        std::fs::write(filename, content).map_err(|e| format!("Failed to write to file: {}", e))?;
        Ok(format!("Successfully wrote to {}", filename))
    }

    fn handle_append(&self, args: &[String]) -> Result<String, String> {
        if args.len() != 2 {
            return Err("Usage: append <filename> <content>".to_string());
        }
        let filename = &args[0];
        let content = &args[1];

        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(filename)
            .map_err(|e| format!("Failed to open file for appending: {}", e))?;

        writeln!(file, "{}", content).map_err(|e| format!("Failed to append to file: {}", e))?;
        Ok(format!("Successfully appended to {}", filename))
    }

    fn handle_read(&self, args: &[String]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("Usage: read <filename>".to_string());
        }
        let filename = &args[0];

        let content =
            std::fs::read_to_string(filename).map_err(|e| format!("Failed to read file: {}", e))?;
        Ok(content)
    }

    fn handle_read_lines(&self, args: &[String]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("Usage: read_lines <filename>".to_string());
        }
        self.handle_read(args)
    }

    fn handle_delete(&self, args: &[String]) -> Result<String, String> {
        if args.len() != 1 {
            return Err("Usage: delete <filename>".to_string());
        }
        let filename = &args[0];

        std::fs::remove_file(filename).map_err(|e| format!("Failed to delete file: {}", e))?;
        Ok(format!("Successfully deleted {}", filename))
    }
}
