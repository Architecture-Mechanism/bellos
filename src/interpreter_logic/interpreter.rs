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

use crate::interpreter_logic::logic::Logic;
use crate::utilities::utilities::ASTNode;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Interpreter {
    pub variables: HashMap<String, String>,
    pub functions: HashMap<String, ASTNode>,
    pub logic: Logic,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            functions: HashMap::new(),
            logic: Logic::new(),
        }
    }

    pub fn interpret(&mut self, nodes: Vec<ASTNode>) -> Result<(), String> {
        for node in nodes {
            if let Err(e) = self.interpret_node(&node) {
                eprintln!("Error executing command: {}", e);
            }
        }
        Ok(())
    }

    pub fn interpret_node(&mut self, node: &ASTNode) -> Result<Option<i32>, String> {
        match node {
            ASTNode::Assignment { name, value } => self.assignment(name, value),
            ASTNode::Block(statements) => self.execute_block(statements),
            ASTNode::If {
                condition,
                then_block,
                else_block,
            } => self.execute_if(condition, then_block, else_block),
            ASTNode::While { condition, block } => self.execute_while(condition, block),
            ASTNode::For { var, list, block } => self.execute_for(var, list, block),
            ASTNode::Case { var, cases } => self.execute_case(var, cases),
            ASTNode::Comparison { left, op, right } => self.execute_comparison(left, op, right),
            ASTNode::Expression(expr) => self.execute_expression(expr),
            ASTNode::Function { name, body } => self.define_function(name, body),
            _ => Err(format!("Unsupported node type in Interpreter: {:?}", node)),
        }
    }

    fn assignment(&mut self, name: &str, value: &str) -> Result<Option<i32>, String> {
        let expanded_value = self.expand_variables(value);
        self.variables.insert(name.to_string(), expanded_value);
        Ok(None)
    }

    fn execute_block(&mut self, statements: &[ASTNode]) -> Result<Option<i32>, String> {
        let mut last_result = Ok(None);
        for statement in statements {
            last_result = self.interpret_node(statement);
            if last_result.is_err() {
                break;
            }
        }
        last_result
    }

    fn execute_if(
        &mut self,
        condition: &ASTNode,
        then_block: &ASTNode,
        else_block: &Option<Box<ASTNode>>,
    ) -> Result<Option<i32>, String> {
        if self.logic.evaluate_condition(&self.variables, condition)? {
            self.interpret_node(then_block)
        } else if let Some(else_block) = else_block {
            self.interpret_node(else_block)
        } else {
            Ok(None)
        }
    }

    fn execute_while(
        &mut self,
        condition: &ASTNode,
        block: &ASTNode,
    ) -> Result<Option<i32>, String> {
        while self.logic.evaluate_condition(&self.variables, condition)? {
            self.interpret_node(block)?;
        }
        Ok(None)
    }

    fn execute_for(
        &mut self,
        var: &str,
        list: &[String],
        block: &ASTNode,
    ) -> Result<Option<i32>, String> {
        for item in list {
            let expanded_item = self.expand_variables(item);
            self.variables.insert(var.to_string(), expanded_item);
            self.interpret_node(block)?;
        }
        Ok(None)
    }

    fn execute_case(
        &mut self,
        var: &ASTNode,
        cases: &[(ASTNode, ASTNode)],
    ) -> Result<Option<i32>, String> {
        let var_str = match var {
            ASTNode::Expression(expr) => self.expand_variables(expr),
            _ => return Err("Invalid case variable".to_string()),
        };
        for (pattern, block) in cases {
            let expanded_pattern = match pattern {
                ASTNode::Expression(expr) => self.expand_variables(expr),
                _ => return Err("Invalid case pattern".to_string()),
            };
            if expanded_pattern == "*" || expanded_pattern == var_str {
                return self.interpret_node(block);
            }
        }
        Ok(None)
    }

    fn execute_comparison(
        &mut self,
        left: &str,
        op: &str,
        right: &str,
    ) -> Result<Option<i32>, String> {
        let result = self
            .logic
            .compare_values(&self.variables, left, op, right)?;
        Ok(Some(if result { 0 } else { 1 }))
    }

    fn execute_expression(&mut self, expr: &str) -> Result<Option<i32>, String> {
        let expanded = self.expand_variables(expr);
        Ok(Some(self.logic.evaluate_arithmetic(&expanded)?))
    }

    fn define_function(&mut self, name: &str, body: &ASTNode) -> Result<Option<i32>, String> {
        self.functions.insert(name.to_string(), body.clone());
        Ok(None)
    }

    pub fn expand_variables(&self, input: &str) -> String {
        self.logic.expand_variables(&self.variables, input)
    }

    pub fn call_function(&mut self, name: &str, args: &[String]) -> Result<Option<i32>, String> {
        if let Some(function_body) = self.functions.get(name).cloned() {
            // Save current variables
            let saved_variables = self.variables.clone();

            // Set up function arguments as variables
            if let ASTNode::Function { name: _, body } = function_body {
                if let ASTNode::Block(statements) = *body {
                    // Assume the first statement is a parameter list
                    if let Some(ASTNode::Assignment {
                        name: params,
                        value: _,
                    }) = statements.first()
                    {
                        let param_names: Vec<&str> = params.split_whitespace().collect();
                        for (i, param_name) in param_names.iter().enumerate() {
                            if i < args.len() {
                                self.variables
                                    .insert(param_name.to_string(), args[i].clone());
                            } else {
                                self.variables.insert(param_name.to_string(), String::new());
                            }
                        }
                    }

                    // Execute function body
                    let result = self.execute_block(&statements[1..]);

                    // Restore original variables
                    self.variables = saved_variables;

                    result
                } else {
                    Err("Invalid function body".to_string())
                }
            } else {
                Err("Invalid function definition".to_string())
            }
        } else {
            Err(format!("Function '{}' not found", name))
        }
    }
}
