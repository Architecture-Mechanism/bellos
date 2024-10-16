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

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    String(String),
    Assignment,
    Pipe,
    Redirect(RedirectType),
    Semicolon,
    NewLine,
    Ampersand,
    LeftParen,
    RightParen,
    If,
    Then,
    Else,
    Fi,
    While,
    Do,
    Done,
    For,
    In,
    Case,
    Esac,
    Function,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectType {
    Out,
    Append,
    In,
}

impl RedirectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RedirectType::Out => ">",
            RedirectType::Append => ">>",
            RedirectType::In => "<",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ASTNode {
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
        direction: RedirectType,
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
    Case {
        var: String,
        cases: Vec<(String, ASTNode)>,
    },
}

impl ASTNode {
    pub fn is_empty_command(&self) -> bool {
        match self {
            ASTNode::Command { name, args } => name.is_empty() && args.is_empty(),
            _ => false,
        }
    }
}
