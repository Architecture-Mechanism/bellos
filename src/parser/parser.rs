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

use crate::utilities::utilities::{ASTNode, RedirectType, Token};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    recursion_depth: usize,
    max_recursion_depth: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
            recursion_depth: 0,
            max_recursion_depth: 1000,
        }
    }

    fn increment_recursion(&mut self) -> Result<(), String> {
        self.recursion_depth += 1;
        if self.recursion_depth > self.max_recursion_depth {
            Err("Maximum recursion depth exceeded".to_string())
        } else {
            Ok(())
        }
    }

    fn decrement_recursion(&mut self) {
        self.recursion_depth -= 1;
    }

    pub fn parse(&mut self) -> Result<Vec<ASTNode>, String> {
        let mut nodes = Vec::new();
        while self.position < self.tokens.len() {
            nodes.push(self.parse_statement()?);
            self.consume_if(Token::Semicolon);
            self.consume_if(Token::NewLine);
        }
        Ok(nodes)
    }

    fn parse_statement(&mut self) -> Result<ASTNode, String> {
        self.increment_recursion()?;
        let result = if self.position >= self.tokens.len() {
            Err("Unexpected end of input".to_string())
        } else {
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
        };
        self.decrement_recursion();
        result
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
            let value = if self.position < self.tokens.len() {
                match &self.tokens[self.position] {
                    Token::Word(w) => w.clone(),
                    Token::String(s) => s.clone(),
                    _ => String::new(), // Allow empty assignments
                }
            } else {
                String::new()
            };
            if self.position < self.tokens.len() {
                self.position += 1;
            }
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
                        | Token::Assignment
                )
            {
                match &self.tokens[self.position] {
                    Token::Word(w) => args.push(w.clone()),
                    Token::String(s) => args.push(s.clone()),
                    _ => break,
                }
                self.position += 1;
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
                let target = if self.position < self.tokens.len() {
                    match &self.tokens[self.position] {
                        Token::Word(w) => w.clone(),
                        Token::String(s) => s.clone(),
                        _ => {
                            return Err(format!(
                                "Expected word after redirect, found {:?}",
                                self.tokens[self.position]
                            ))
                        }
                    }
                } else {
                    return Err("Unexpected end of input after redirect".to_string());
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
        self.expect_token(Token::RightParen)?;
        Ok(ASTNode::Block(statements))
    }

    fn parse_if(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'if'
        let condition = Box::new(self.parse_command()?);
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
        let condition = Box::new(self.parse_command()?);
        self.expect_token(Token::Do)?;
        let block = Box::new(self.parse_block()?);
        self.expect_token(Token::Done)?;
        Ok(ASTNode::While { condition, block })
    }

    fn parse_for(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'for'
        let var = match &self.tokens[self.position] {
            Token::Word(w) => w.clone(),
            _ => return Err("Expected variable name after 'for'".to_string()),
        };
        self.position += 1;
        self.expect_token(Token::In)?;
        let mut list = Vec::new();
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::Do {
            match &self.tokens[self.position] {
                Token::Word(w) => list.push(w.clone()),
                Token::String(s) => list.push(s.clone()),
                _ => break,
            }
            self.position += 1;
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

    fn parse_command(&mut self) -> Result<ASTNode, String> {
        let mut args = Vec::new();
        while self.position < self.tokens.len()
            && !matches!(
                self.tokens[self.position],
                Token::Then | Token::Do | Token::Done | Token::Fi | Token::Else
            )
        {
            match &self.tokens[self.position] {
                Token::Word(w) => args.push(w.clone()),
                Token::String(s) => args.push(s.clone()),
                _ => break,
            }
            self.position += 1;
        }
        if args.is_empty() {
            Err("Expected command".to_string())
        } else {
            Ok(ASTNode::Command {
                name: args[0].clone(),
                args: args[1..].to_vec(),
            })
        }
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), String> {
        if self.position < self.tokens.len() && self.tokens[self.position] == expected {
            self.position += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.tokens.get(self.position).unwrap_or(&Token::NewLine)
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
