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

use crate::utilities::utilities::{ASTNode, Token};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<ASTNode>, String> {
        let mut nodes = Vec::new();
        while self.position < self.tokens.len() {
            self.skip_newlines();
            if self.position >= self.tokens.len() {
                break;
            }
            nodes.push(self.parse_statement()?);
        }
        Ok(nodes)
    }

    fn parse_statement(&mut self) -> Result<ASTNode, String> {
        if self.position >= self.tokens.len() {
            return Err("Unexpected end of input".to_string());
        }
        match &self.tokens[self.position] {
            Token::Word(w) if w.eq_ignore_ascii_case("if") => self.parse_if(),
            Token::Word(w) if w.eq_ignore_ascii_case("while") => self.parse_while(),
            Token::Word(w) if w.eq_ignore_ascii_case("for") => self.parse_for(),
            Token::Word(w) if w.eq_ignore_ascii_case("case") => self.parse_case(),
            Token::Word(w) if w.eq_ignore_ascii_case("function") => self.parse_function(),
            _ => self.parse_command_or_assignment(),
        }
    }

    fn parse_if(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'if'
        let condition = Box::new(self.parse_command()?);
        self.skip_newlines_and_expect("then")?;
        let then_block = Box::new(self.parse_block("else", "fi")?);
        let else_block = if self.current_token_is("else") {
            self.position += 1;
            self.skip_newlines();
            Some(Box::new(self.parse_block("fi", "fi")?))
        } else {
            None
        };
        self.skip_newlines_and_expect("fi")?;
        Ok(ASTNode::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_while(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'while'
        let condition = Box::new(self.parse_command()?);
        self.skip_newlines_and_expect("do")?;
        let block = Box::new(self.parse_block("done", "done")?);
        self.skip_newlines_and_expect("done")?;
        Ok(ASTNode::While { condition, block })
    }

    fn parse_for(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'for'
        let var = self.expect_word()?;
        self.skip_newlines_and_expect("in")?;
        let list = self.parse_list()?;
        self.skip_newlines_and_expect("do")?;
        let block = Box::new(self.parse_block("done", "done")?);
        self.skip_newlines_and_expect("done")?;
        Ok(ASTNode::For { var, list, block })
    }

    fn parse_case(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'case'
        let var = self.parse_expression()?;
        self.skip_newlines_and_expect("in")?;
        let mut cases = Vec::new();
        while !self.current_token_is("esac") {
            self.skip_newlines();
            let pattern = self.parse_expression()?;
            self.expect_token(&Token::RightParen)?;
            let block = self.parse_block(";;", "esac")?;
            cases.push((pattern, block));
            self.skip_newlines();
            if self.current_token_is(";;") {
                self.position += 1;
            }
        }
        self.skip_newlines_and_expect("esac")?;
        Ok(ASTNode::Case { var, cases })
    }

    fn parse_function(&mut self) -> Result<ASTNode, String> {
        self.position += 1; // Consume 'function'
        let name = self.expect_word()?;
        self.skip_newlines();
        self.expect_token(&Token::LeftParen)?;
        self.skip_newlines();
        let body = Box::new(self.parse_block(")", ")")?);
        self.expect_token(&Token::RightParen)?;
        Ok(ASTNode::Function { name, body })
    }

    fn parse_block(&mut self, end_token1: &str, end_token2: &str) -> Result<ASTNode, String> {
        let mut statements = Vec::new();
        while !self.current_token_is(end_token1) && !self.current_token_is(end_token2) {
            self.skip_newlines();
            if self.current_token_is(end_token1) || self.current_token_is(end_token2) {
                break;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(ASTNode::Block(statements))
    }

    fn parse_command(&mut self) -> Result<ASTNode, String> {
        let mut args = Vec::new();
        while self.position < self.tokens.len() && !self.is_command_end() {
            args.push(self.expect_word()?);
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

    fn parse_list(&mut self) -> Result<Vec<String>, String> {
        let mut list = Vec::new();
        while !self.current_token_is("do") {
            list.push(self.expect_word()?);
            self.skip_newlines();
        }
        Ok(list)
    }

    fn parse_expression(&mut self) -> Result<String, String> {
        self.expect_word()
    }

    fn expect_word(&mut self) -> Result<String, String> {
        if self.position >= self.tokens.len() {
            return Err("Unexpected end of input".to_string());
        }
        match &self.tokens[self.position] {
            Token::Word(w) | Token::String(w) => {
                self.position += 1;
                Ok(w.clone())
            }
            _ => Err(format!(
                "Expected word, found {:?}",
                self.tokens[self.position]
            )),
        }
    }

    fn expect_token(&mut self, expected: &Token) -> Result<(), String> {
        if self.position >= self.tokens.len() {
            return Err(format!("Expected {:?}, found end of input", expected));
        }
        if self.tokens[self.position] == *expected {
            self.position += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, found {:?}",
                expected, self.tokens[self.position]
            ))
        }
    }

    fn current_token_is(&self, token: &str) -> bool {
        if self.position >= self.tokens.len() {
            return false;
        }
        match &self.tokens[self.position] {
            Token::Word(w) => w.eq_ignore_ascii_case(token),
            _ => false,
        }
    }

    fn skip_newlines(&mut self) {
        while self.position < self.tokens.len() && self.tokens[self.position] == Token::NewLine {
            self.position += 1;
        }
    }

    fn skip_newlines_and_expect(&mut self, expected: &str) -> Result<(), String> {
        self.skip_newlines();
        if self.position >= self.tokens.len() {
            return Err(format!("Expected {}, found end of input", expected));
        }
        if self.current_token_is(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected {}, found {:?}",
                expected, self.tokens[self.position]
            ))
        }
    }

    fn is_command_end(&self) -> bool {
        self.position >= self.tokens.len()
            || matches!(
                self.tokens[self.position],
                Token::Semicolon | Token::NewLine
            )
            || self.current_token_is("then")
            || self.current_token_is("do")
            || self.current_token_is("done")
            || self.current_token_is("fi")
            || self.current_token_is("else")
            || self.current_token_is("elif")
            || self.current_token_is("esac")
    }

    fn parse_command_or_assignment(&mut self) -> Result<ASTNode, String> {
        let name = self.expect_word()?;
        if self.position < self.tokens.len() && self.tokens[self.position] == Token::Assignment {
            self.position += 1;
            let value = self.parse_expression()?;
            Ok(ASTNode::Assignment { name, value })
        } else {
            let mut args = Vec::new();
            while self.position < self.tokens.len() && !self.is_command_end() {
                args.push(self.expect_word()?);
            }
            Ok(ASTNode::Command { name, args })
        }
    }
}
