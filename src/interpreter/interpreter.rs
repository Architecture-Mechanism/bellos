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

use crate::utilities::utilities::ASTNode;
use std::collections::HashMap;
use std::env;

pub struct Interpreter {
    pub variables: HashMap<String, String>,
    pub functions: HashMap<String, ASTNode>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            self.interpret_node(Box::new(node))?;
        }
        Ok(())
    }

    pub fn interpret_node(&mut self, node: Box<ASTNode>) -> Result<Option<i32>, String> {
        match *node {
            ASTNode::Assignment { name, value } => {
                let expanded_value = self.expand_variables(&value);
                self.variables.insert(name, expanded_value);
                Ok(None)
            }
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
                if self.evaluate_condition(&condition)? {
                    self.interpret_node(then_block)?;
                } else if let Some(else_block) = else_block {
                    self.interpret_node(else_block)?;
                }
                Ok(None)
            }
            ASTNode::While { condition, block } => {
                while self.evaluate_condition(&condition)? {
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
            _ => Err("Node type not handled by Interpreter".to_string()),
        }
    }

    pub fn evaluate_condition(&mut self, condition: &ASTNode) -> Result<bool, String> {
        match condition {
            ASTNode::Command { name, args } => {
                let expanded_args: Vec<String> =
                    args.iter().map(|arg| self.expand_variables(arg)).collect();
                match name.as_str() {
                    "[" | "test" => {
                        if expanded_args.len() < 3 || expanded_args.last() != Some(&"]".to_string())
                        {
                            return Err("Invalid test condition".to_string());
                        }
                        match expanded_args[1].as_str() {
                            "-eq" => Ok(expanded_args[0] == expanded_args[2]),
                            "-ne" => Ok(expanded_args[0] != expanded_args[2]),
                            "-lt" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                < expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-le" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                <= expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-gt" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                > expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-ge" => Ok(expanded_args[0].parse::<i32>().unwrap_or(0)
                                >= expanded_args[2].parse::<i32>().unwrap_or(0)),
                            "-z" => Ok(expanded_args[0].is_empty()),
                            "-n" => Ok(!expanded_args[0].is_empty()),
                            _ => Err(format!("Unsupported test condition: {}", expanded_args[1])),
                        }
                    }
                    _ => Err("Condition evaluation not supported for this command".to_string()),
                }
            }
            _ => Err("Invalid condition node".to_string()),
        }
    }

    pub fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'(') {
                    chars.next(); // consume '('
                    let expr: String = chars.by_ref().take_while(|&c| c != ')').collect();
                    if expr.starts_with('(') && expr.ends_with(')') {
                        // Arithmetic expression
                        let arithmetic_expr = &expr[1..expr.len() - 1];
                        match self.evaluate_arithmetic(arithmetic_expr) {
                            Ok(value) => result.push_str(&value.to_string()),
                            Err(e) => result.push_str(&format!("Error: {}", e)),
                        }
                    }
                } else {
                    let var_name: String = chars
                        .by_ref()
                        .take_while(|&c| c.is_alphanumeric() || c == '_')
                        .collect();
                    if let Some(value) = self.variables.get(&var_name) {
                        result.push_str(value);
                    } else if let Ok(value) = env::var(&var_name) {
                        result.push_str(&value);
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    fn evaluate_arithmetic(&self, expr: &str) -> Result<i32, String> {
        let tokens: Vec<&str> = expr.split_whitespace().collect();
        if tokens.len() != 3 {
            return Err("Invalid arithmetic expression".to_string());
        }

        let a = self.get_var_value(tokens[0])?;
        let b = self.get_var_value(tokens[2])?;

        match tokens[1] {
            "+" => Ok(a + b),
            "-" => Ok(a - b),
            "*" => Ok(a * b),
            "/" => {
                if b != 0 {
                    Ok(a / b)
                } else {
                    Err("Division by zero".to_string())
                }
            }
            "%" => {
                if b != 0 {
                    Ok(a % b)
                } else {
                    Err("Modulo by zero".to_string())
                }
            }
            _ => Err(format!("Unsupported operation: {}", tokens[1])),
        }
    }

    fn get_var_value(&self, var: &str) -> Result<i32, String> {
        if let Some(value) = self.variables.get(var) {
            value
                .parse()
                .map_err(|_| format!("Invalid integer: {}", value))
        } else if let Ok(value) = env::var(var) {
            value
                .parse()
                .map_err(|_| format!("Invalid integer: {}", value))
        } else {
            var.parse()
                .map_err(|_| format!("Invalid integer or undefined variable: {}", var))
        }
    }
}
