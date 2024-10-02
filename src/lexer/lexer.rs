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

use crate::utilities::utilities::Token;

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
            "for" => Token::For,
            "do" => Token::Do,
            "done" => Token::Done,
            "in" => Token::In,
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

impl Iterator for Lexer {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
