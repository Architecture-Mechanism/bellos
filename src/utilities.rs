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

use std::fs::File;
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Assignment,
    Pipe,
    Redirect(String),
    LeftParen,
    RightParen,
    Semicolon,
    NewLine,
    If,
    Then,
    Else,
    Fi,
    While,
    Do,
    Done,
    For,
    In,
    Function,
    Ampersand,
}

#[derive(Debug, Clone)]
enum ASTNode {
    Command {
        name: String,
        args: Vec<String>,
    },
    Assignment {
        name: String,
        value: String,
    },
    Pipeline(Vec<ASTNode>),
    Redirect {
        node: Box<ASTNode>,
        direction: String,
        target: String,
    },
    Block(Vec<ASTNode>),
    If {
        condition: Box<ASTNode>,
        then_block: Box<ASTNode>,
        else_block: Option<Box<ASTNode>>,
    },
    While {
        condition: Box<ASTNode>,
        block: Box<ASTNode>,
    },
    For {
        var: String,
        list: Vec<String>,
        block: Box<ASTNode>,
    },
    Function {
        name: String,
        body: Box<ASTNode>,
    },
    Background(Box<ASTNode>),
}

fn execute_script(interpreter: &mut Interpreter, filename: &str) -> Result<(), String> {
    let file =
        File::open(filename).map_err(|e| format!("Error opening file {}: {}", filename, e))?;
    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();

    // Check for shebang
    if let Some(Ok(first_line)) = lines.next() {
        if !first_line.starts_with("#!") {
            // If no shebang, process this line
            process_line(interpreter, &first_line, 1)?;
        }
    }

    // Process remaining lines
    for (line_num, line) in lines.enumerate() {
        let line = line.map_err(|e| format!("Error reading line: {}", e))?;
        process_line(interpreter, &line, line_num + 2)?;
    }

    Ok(())
}

fn process_line(interpreter: &mut Interpreter, line: &str, line_num: usize) -> Result<(), String> {
    let trimmed_line = line.trim();
    if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
        return Ok(()); // Skip empty lines and comments
    }

    let lexer = Lexer::new(line.to_string());
    let tokens: Vec<Token> = lexer.into_iter().collect();
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(ast) => {
            if let Err(e) = interpreter.interpret(ast) {
                eprintln!("Error on line {}: {}", line_num, e);
            }
        }
        Err(e) => eprintln!("Parse error on line {}: {}", line_num, e),
    }
    Ok(())
}

fn run_interactive_mode(interpreter: &mut Interpreter) -> Result<(), String> {
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
                if let Err(e) = interpreter.interpret(ast) {
                    eprintln!("Error: {}", e);
                }
            }
            Err(e) => eprintln!("Parse error: {}", e),
        }
    }
}
