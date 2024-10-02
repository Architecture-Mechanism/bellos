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
use crate::utilities::utilities::Token;
use std::fs::File;
use std::io::{self, BufRead, Write};

pub struct Executor {
    interpreter: Interpreter,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
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
        let mut lines = reader.lines();

        // Check for shebang
        if let Some(Ok(first_line)) = lines.next() {
            if !first_line.starts_with("#!") {
                // If no shebang, process this line
                self.process_line(&first_line, 1)?;
            }
        }

        // Process remaining lines
        for (line_num, line) in lines.enumerate() {
            let line = line.map_err(|e| format!("Error reading line: {}", e))?;
            self.process_line(&line, line_num + 2)?;
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
                if let Err(e) = self.interpreter.interpret(ast) {
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
                    if let Err(e) = self.interpreter.interpret(ast) {
                        eprintln!("Error: {}", e);
                    }
                }
                Err(e) => eprintln!("Parse error: {}", e),
            }
        }
    }
}
