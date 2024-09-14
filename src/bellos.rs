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

use glob::glob;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

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

struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    fn new(input: String) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return None;
        }

        match self.input[self.position] {
            '=' => {
                self.position += 1;
                Some(Token::Assignment)
            }
            '|' => {
                self.position += 1;
                Some(Token::Pipe)
            }
            '>' => {
                self.position += 1;
                Some(Token::Redirect(">".to_string()))
            }
            '<' => {
                self.position += 1;
                Some(Token::Redirect("<".to_string()))
            }
            '(' => {
                self.position += 1;
                Some(Token::LeftParen)
            }
            ')' => {
                self.position += 1;
                Some(Token::RightParen)
            }
            ';' => {
                self.position += 1;
                Some(Token::Semicolon)
            }
            '\n' => {
                self.position += 1;
                Some(Token::NewLine)
            }
            '&' => {
                self.position += 1;
                Some(Token::Ampersand)
            }
            '"' => Some(self.read_string()),
            _ => Some(self.read_word()),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.input[self.position].is_whitespace() {
            self.position += 1;
        }
    }

    fn read_word(&mut self) -> Token {
        let start = self.position;
        while self.position < self.input.len()
            && !self.input[self.position].is_whitespace()
            && !matches!(
                self.input[self.position],
                '=' | '|' | '>' | '<' | '(' | ')' | ';' | '&' | '\n'
            )
        {
            self.position += 1;
        }
        let word: String = self.input[start..self.position].iter().collect();
        match word.as_str() {
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "fi" => Token::Fi,
            "while" => Token::While,
            "do" => Token::Do,
            "done" => Token::Done,
            "for" => Token::For,
            "in" => Token::In,
            "function" => Token::Function,
            _ => Token::Word(word),
        }
    }

    fn read_string(&mut self) -> Token {
        self.position += 1; // Skip opening quote
        let start = self.position;
        while self.position < self.input.len() && self.input[self.position] != '"' {
            self.position += 1;
        }
        let result = Token::Word(self.input[start..self.position].iter().collect());
        self.position += 1; // Skip closing quote
        result
    }
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn parse(&mut self) -> Result<Vec<ASTNode>, String> {
        let mut nodes = Vec::new();
        while self.position < self.tokens.len() {
            nodes.push(self.parse_statement()?);
            self.consume_if(Token::Semicolon);
            self.consume_if(Token::NewLine);
        }
        Ok(nodes)
    }

    fn parse_statement(&mut self) -> Result<ASTNode, String> {
        match &self.tokens[self.position] {
            Token::Word(_) => self.parse_command_or_assignment(),
            Token::LeftParen => self.parse_block(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Function => self.parse_function(),
            _ => Err(format!(
                "Unexpected token: {:?}",
                self.tokens[self.position]
            )),
        }
    }

    fn parse_command_or_assignment(&mut self) -> Result<ASTNode, String> {
        let name = match &self.tokens[self.position] {
            Token::Word(w) => w.clone(),
            _ => {
                return Err(format!(
                    "Expected word, found {:?}",
                    self.tokens[self.position]
                ))
            }
        };
        self.position += 1;

        if self.position < self.tokens.len() && self.tokens[self.position] == Token::Assignment {
            self.position += 1;
            let value = match &self.tokens[self.position] {
                Token::Word(w) => w.clone(),
                _ => {
                    return Err(format!(
                        "Expected word after assignment, found {:?}",
                        self.tokens[self.position]
                    ))
                }
            };
            self.position += 1;
            Ok(ASTNode::Assignment { name, value })
        } else {
            let mut args = Vec::new();
            while self.position < self.tokens.len()
                && !matches!(
                    self.tokens[self.position],
                    Token::Pipe
                        | Token::Redirect(_)
                        | Token::Semicolon
                        | Token::NewLine
                        | Token::Ampersand
                )
            {
                if let Token::Word(w) = &self.tokens[self.position] {
                    args.push(w.clone());
                    self.position += 1;
                } else {
                    break;
                }
            }
            let command = ASTNode::Command { name, args };
            self.parse_pipeline_or_redirect(command)
        }
    }

    fn parse_pipeline_or_redirect(&mut self, left: ASTNode) -> Result<ASTNode, String> {
        if self.position >= self.tokens.len() {
            return Ok(left);
        }

        match &self.tokens[self.position] {
            Token::Pipe => {
                self.position += 1;
                let right = self.parse_command_or_assignment()?;
                let pipeline = ASTNode::Pipeline(vec![left, right]);
                self.parse_pipeline_or_redirect(pipeline)
            }
            Token::Redirect(direction) => {
                self.position += 1;
                let target = match &self.tokens[self.position] {
                    Token::Word(w) => w.clone(),
                    _ => {
                        return Err(format!(
                            "Expected word after redirect, found {:?}",
                            self.tokens[self.position]
                        ))
                    }
                };
                self.position += 1;
                let redirect = ASTNode::Redirect {
                    node: Box::new(left),
                    direction: direction.clone(),
                    target,
                };
                self.parse_pipeline_or_redirect(redirect)
            }
            Token::Ampersand => {
                self.position += 1;
                Ok(ASTNode::Background(Box::new(left)))
            }
            _ => Ok(left),
        }
    }

    fn parse_block(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume left paren
        let mut statements = Vec::new();
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::RightParen {
            statements.push(self.parse_statement()?);
            self.consume_if(Token::Semicolon);
            self.consume_if(Token::NewLine);
        }
        self.position += 1; // Consume right paren
        Ok(ASTNode::Block(statements))
    }

    fn parse_if(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'if'
        let condition = Box::new(self.parse_statement()?);
        self.expect_token(Token::Then)?;
        let then_block = Box::new(self.parse_block()?);
        let else_block = if self.consume_if(Token::Else) {
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };
        self.expect_token(Token::Fi)?;
        Ok(ASTNode::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_while(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'while'
        let condition = Box::new(self.parse_statement()?);
        self.expect_token(Token::Do)?;
        let block = Box::new(self.parse_block()?);
        self.expect_token(Token::Done)?;
        Ok(ASTNode::While { condition, block })
    }

    fn parse_for(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'for'
        let var = match &self.tokens[self.position] {
            Token::Word(w) => w.clone(),
            _ => {
                return Err(format!(
                    "Expected variable name after 'for', found {:?}",
                    self.tokens[self.position]
                ))
            }
        };
        self.position += 1;
        self.expect_token(Token::In)?;
        let mut list = Vec::new();
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::Do {
            if let Token::Word(w) = &self.tokens[self.position] {
                list.push(w.clone());
                self.position += 1;
            } else {
                break;
            }
        }
        self.expect_token(Token::Do)?;
        let block = Box::new(self.parse_block()?);
        self.expect_token(Token::Done)?;
        Ok(ASTNode::For { var, list, block })
    }

    fn parse_function(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'function'
        let name = match &self.tokens[self.position] {
            Token::Word(w) => w.clone(),
            _ => {
                return Err(format!(
                    "Expected function name, found {:?}",
                    self.tokens[self.position]
                ))
            }
        };
        self.position += 1;
        let body = Box::new(self.parse_block()?);
        Ok(ASTNode::Function { name, body })
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), String> {
        if self.position < self.tokens.len() && self.tokens[self.position] == expected {
            self.position += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.tokens.get(self.position)
            ))
        }
    }

    fn consume_if(&mut self, token: Token) -> bool {
        if self.position < self.tokens.len() && self.tokens[self.position] == token {
            self.position += 1;
            true
        } else {
            false
        }
    }
}

struct Interpreter {
    variables: HashMap<String, String>,
    functions: HashMap<String, ASTNode>,
    background_jobs: Arc<Mutex<Vec<Child>>>,
}

impl Interpreter {
    fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            functions: HashMap::new(),
            background_jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn interpret(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            self.interpret_node(Box::new(node))?;
        }
        Ok(())
    }

    fn interpret_node(&mut self, node: Box<ASTNode>) -> Result<Option<i32>, String> {
        match *node {
            ASTNode::Command { name, args } => self.execute_command(name, args),
            ASTNode::Assignment { name, value } => {
                self.variables.insert(name, self.expand_variables(&value));
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
                if self.interpret_node(Box::new(*condition))? == Some(0) {
                    self.interpret_node(Box::new(*then_block))
                } else if let Some(else_block) = else_block {
                    self.interpret_node(Box::new(*else_block))
                } else {
                    Ok(None)
                }
            }
            ASTNode::While { condition, block } => {
                while self.interpret_node(Box::new(*condition.clone()))? == Some(0) {
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
        let expanded_args: Vec<String> =
            args.iter().map(|arg| self.expand_variables(arg)).collect();
        match name.as_str() {
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
                if let Some(func) = self.functions.get(&name) {
                    return self.interpret_node(Box::new(func.clone()));
                }
                let expanded_name = self.expand_variables(&name);
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
        direction: String,
        target: String,
    ) -> Result<Option<i32>, String> {
        let target = self.expand_variables(&target);
        match direction.as_str() {
            ">" => {
                let mut file = File::create(&target).map_err(|e| e.to_string())?;
                let result = self.capture_output(Box::new(node))?;
                file.write_all(result.as_bytes())
                    .map_err(|e| e.to_string())?;
                print!("{}", result);
                Ok(Some(0))
            }
            "<" => {
                let mut file = File::open(&target).map_err(|e| e.to_string())?;
                let mut input = String::new();
                file.read_to_string(&mut input).map_err(|e| e.to_string())?;
                self.execute_with_input(Box::new(node), input)
            }
            _ => Err(format!("Unsupported redirection: {}", direction)),
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
        // Create a temporary file to store the input
        let mut temp_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        temp_file
            .write_all(input.as_bytes())
            .map_err(|e| e.to_string())?;
        temp_file.flush().map_err(|e| e.to_string())?;

        // Reopen the temp file as read-only
        let input_file = File::open(temp_file.path()).map_err(|e| e.to_string())?;

        // Replace stdin with our temp file
        let stdin = io::stdin();
        let old_stdin = stdin.lock();
        let new_stdin = unsafe {
            use std::os::unix::io::FromRawFd;
            std::fs::File::from_raw_fd(input_file.as_raw_fd())
        };

        // Execute the node with the new stdin
        let result = self.interpret_node(node);

        // Restore the old stdin
        drop(new_stdin);
        drop(old_stdin);

        result
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' && chars.peek().map_or(false, |&next| next != ' ') {
                let var_name: String = chars
                    .by_ref()
                    .take_while(|&c| c.is_alphanumeric() || c == '_')
                    .collect();
                if var_name == "*" || var_name == "@" {
                    // Expand to all arguments
                    result.push_str(&env::args().skip(1).collect::<Vec<String>>().join(" "));
                } else if let Ok(value) = env::var(&var_name) {
                    result.push_str(&value);
                } else if let Some(value) = self.variables.get(&var_name) {
                    result.push_str(value);
                }
            } else if c == '~' && (chars.peek().is_none() || chars.peek() == Some(&'/')) {
                if let Ok(home) = env::var("HOME") {
                    result.push_str(&home);
                } else {
                    result.push(c);
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

fn main() -> Result<(), String> {
    let mut interpreter = Interpreter::new();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Execute script file
        let filename = &args[1];
        let content = fs::read_to_string(filename)
            .map_err(|e| format!("Error reading file {}: {}", filename, e))?;
        let lexer = Lexer::new(content);
        let tokens: Vec<Token> = lexer.into_iter().collect();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        interpreter.interpret(ast)?;
    } else {
        // Interactive mode
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
    Ok(())
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
