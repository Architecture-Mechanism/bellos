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
            ASTNode::Command { name: _, args: _ } => {
                Err("Commands should be handled by Processes".to_string())
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
                    let mut depth = 0;
                    let mut expr = String::new();
                    for c in chars.by_ref() {
                        expr.push(c);
                        if c == '(' {
                            depth += 1;
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                    if expr.starts_with("((") && expr.ends_with("))") {
                        match self.evaluate_arithmetic(&expr) {
                            Ok(value) => result.push_str(&value.to_string()),
                            Err(e) => result.push_str(&format!("Error: {}", e)),
                        }
                    } else {
                        result.push('$');
                        result.push_str(&expr);
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
                    } else {
                        result.push('$');
                        result.push_str(&var_name);
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    pub fn evaluate_arithmetic(&self, expr: &str) -> Result<i32, String> {
        let expr = expr.trim();
        let inner_expr = if expr.starts_with("$((") && expr.ends_with("))") {
            &expr[3..expr.len() - 2]
        } else if expr.starts_with("((") && expr.ends_with("))") {
            &expr[2..expr.len() - 2]
        } else {
            expr
        };

        // Handle parentheses
        if inner_expr.contains('(') {
            let mut depth = 0;
            let mut start = 0;
            for (i, c) in inner_expr.chars().enumerate() {
                match c {
                    '(' => {
                        if depth == 0 {
                            start = i + 1;
                        }
                        depth += 1;
                    }
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            let sub_result = self.evaluate_arithmetic(&inner_expr[start..i])?;
                            let new_expr = format!(
                                "{} {} {}",
                                &inner_expr[..start - 1],
                                sub_result,
                                &inner_expr[i + 1..]
                            );
                            return self.evaluate_arithmetic(&new_expr);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Split the expression into tokens
        let tokens: Vec<&str> = inner_expr.split_whitespace().collect();

        // Handle single number or variable
        if tokens.len() == 1 {
            return self.get_var_value(tokens[0]);
        }

        // Handle binary operations
        if tokens.len() == 3 {
            let a = self.get_var_value(tokens[0])?;
            let b = self.get_var_value(tokens[2])?;

            let result = match tokens[1] {
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
            };

            result
        } else {
            Err("Invalid arithmetic expression".to_string())
        }
    }

    fn get_var_value(&self, var: &str) -> Result<i32, String> {
        if let Some(value) = self.variables.get(var) {
            value
                .parse()
                .map_err(|_| format!("Invalid integer: {}", value))
        } else if let Ok(value) = var.parse() {
            Ok(value)
        } else {
            Err(format!("Undefined variable or invalid integer: {}", var))
        }
    }
}
