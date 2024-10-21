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

#[derive(Clone)]
pub struct Logic;

impl Logic {
    pub fn new() -> Self {
        Logic
    }

    pub fn expand_variables(&self, variables: &HashMap<String, String>, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'(') {
                    chars.next(); // Consume '('
                    if chars.peek() == Some(&'(') {
                        chars.next(); // Consume second '('
                        let expr = self.extract_arithmetic_expression(&mut chars);
                        if let Ok(value) = self.evaluate_arithmetic(&expr) {
                            result.push_str(&value.to_string());
                        } else {
                            result.push_str(&format!("$(({})", expr));
                        }
                    } else {
                        let cmd = self.extract_command_substitution(&mut chars);
                        // For now, we'll just insert the command as-is
                        result.push_str(&format!("$({})", cmd));
                    }
                } else {
                    let var_name: String = chars
                        .by_ref()
                        .take_while(|&c| c.is_alphanumeric() || c == '_')
                        .collect();
                    if let Some(value) = variables.get(&var_name) {
                        result.push_str(value);
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

    pub fn extract_arithmetic_expression(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> String {
        let mut expr = String::new();
        let mut depth = 2; // We've already consumed "(("
        while let Some(c) = chars.next() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            expr.push(c);
        }
        expr
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

        self.evaluate_arithmetic_expression(inner_expr)
    }

    fn extract_command_substitution(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> String {
        let mut depth = 1;
        let mut cmd = String::new();
        for c in chars.by_ref() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            cmd.push(c);
        }
        cmd
    }

    fn evaluate_arithmetic_expression(&self, expr: &str) -> Result<i32, String> {
        let tokens: Vec<&str> = expr.split_whitespace().collect();
        if tokens.len() != 3 {
            return Err("Invalid arithmetic expression".to_string());
        }

        let left: i32 = self.parse_value(tokens[0])?;
        let right: i32 = self.parse_value(tokens[2])?;

        match tokens[1] {
            "+" => Ok(left + right),
            "-" => Ok(left - right),
            "*" => Ok(left * right),
            "/" => {
                if right != 0 {
                    Ok(left / right)
                } else {
                    Err("Division by zero".to_string())
                }
            }
            "%" => {
                if right != 0 {
                    Ok(left % right)
                } else {
                    Err("Modulo by zero".to_string())
                }
            }
            _ => Err(format!("Unsupported operation: {}", tokens[1])),
        }
    }

    fn parse_value(&self, value: &str) -> Result<i32, String> {
        value
            .parse()
            .map_err(|_| format!("Invalid integer: {}", value))
    }

    pub fn compare_values(
        &self,
        variables: &HashMap<String, String>,
        left: &str,
        op: &str,
        right: &str,
    ) -> Result<bool, String> {
        let left_val = self.expand_variables(variables, left);
        let right_val = self.expand_variables(variables, right);

        match op {
            "-eq" => Ok(left_val == right_val),
            "-ne" => Ok(left_val != right_val),
            "-lt" => self.compare_numbers(&left_val, &right_val, |a, b| a < b),
            "-le" => self.compare_numbers(&left_val, &right_val, |a, b| a <= b),
            "-gt" => self.compare_numbers(&left_val, &right_val, |a, b| a > b),
            "-ge" => self.compare_numbers(&left_val, &right_val, |a, b| a >= b),
            _ => Err(format!("Unknown comparison operator: {}", op)),
        }
    }

    fn compare_numbers<F>(&self, left: &str, right: &str, compare: F) -> Result<bool, String>
    where
        F: Fn(i32, i32) -> bool,
    {
        let left_num = left
            .parse::<i32>()
            .map_err(|_| format!("Invalid number: {}", left))?;
        let right_num = right
            .parse::<i32>()
            .map_err(|_| format!("Invalid number: {}", right))?;
        Ok(compare(left_num, right_num))
    }

    pub fn evaluate_condition(
        &self,
        variables: &HashMap<String, String>,
        condition: &ASTNode,
    ) -> Result<bool, String> {
        match condition {
            ASTNode::Comparison { left, op, right } => {
                self.compare_values(variables, left, op, right)
            }
            ASTNode::Expression(expr) => {
                let result = self.evaluate_arithmetic(&self.expand_variables(variables, expr))?;
                Ok(result != 0)
            }
            _ => Err("Invalid condition".to_string()),
        }
    }
}
