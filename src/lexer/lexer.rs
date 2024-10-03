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

use crate::utilities::utilities::{RedirectType, Token};

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return None;
        }

        Some(match self.current_char() {
            ' ' | '\t' => {
                self.advance();
                return self.next_token();
            }
            '\n' => {
                self.advance();
                Token::NewLine
            }
            ';' => {
                self.advance();
                Token::Semicolon
            }
            '|' => {
                self.advance();
                Token::Pipe
            }
            '&' => {
                self.advance();
                Token::Ampersand
            }
            '=' => {
                self.advance();
                Token::Assignment
            }
            '(' => {
                self.advance();
                Token::LeftParen
            }
            ')' => {
                self.advance();
                Token::RightParen
            }
            '>' => {
                self.advance();
                if self.current_char() == '>' {
                    self.advance();
                    Token::Redirect(RedirectType::Append)
                } else {
                    Token::Redirect(RedirectType::Out)
                }
            }
            '<' => {
                self.advance();
                Token::Redirect(RedirectType::In)
            }
            '"' => self.read_string(),
            '$' => {
                if self.peek_next() == Some('(') {
                    Token::Word(self.read_command_substitution())
                } else {
                    self.read_word()
                }
            }
            _ => self.read_word(),
        })
    }

    fn current_char(&self) -> char {
        self.input[self.position]
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn peek_next(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && matches!(self.input[self.position], ' ' | '\t') {
            self.advance();
        }
    }

    fn read_word(&mut self) -> Token {
        let start = self.position;
        while self.position < self.input.len()
            && !matches!(
                self.current_char(),
                ' ' | '\t' | '\n' | ';' | '|' | '&' | '=' | '(' | ')' | '>' | '<' | '"'
            )
        {
            self.advance();
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
        self.advance(); // Skip opening quote
        let start = self.position;
        while self.position < self.input.len() && self.current_char() != '"' {
            if self.current_char() == '\\' && self.peek_next() == Some('"') {
                self.advance(); // Skip the backslash
            }
            self.advance();
        }
        let string: String = self.input[start..self.position].iter().collect();
        if self.position < self.input.len() {
            self.advance(); // Skip closing quote
        }
        Token::String(string)
    }

    fn read_command_substitution(&mut self) -> String {
        let mut cmd = String::from("$(");
        self.advance(); // Skip $
        self.advance(); // Skip (
        let mut depth = 1;

        while self.position < self.input.len() && depth > 0 {
            match self.current_char() {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            cmd.push(self.current_char());
            self.advance();
        }
        cmd
    }
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
