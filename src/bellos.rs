mod interpreter;
mod lexer;
mod parser;
mod utilities;

use crate::interpreter::interpreter::Interpreter;
use crate::lexer::lexer::Lexer;
use crate::parser::parser::Parser;
use crate::utilities::utilities::Token;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, Write};

fn main() -> Result<(), String> {
    let mut interpreter = Interpreter::new();
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Execute script file
        let filename = &args[1];
        execute_script(&mut interpreter, filename)?;
    } else {
        // Interactive mode
        run_interactive_mode(&mut interpreter)?;
    }
    Ok(())
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
