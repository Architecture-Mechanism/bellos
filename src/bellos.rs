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

#[allow(dead_code)]
pub mod Interpreter {
    mod interpreter;
}

pub mod Lexer {
    mod lexer;
}

pub mod Parser {
    mod parser;
}
pub mod utilities;
use std::env;

fn main() -> Result<(), String> {
    let mut interpreter = Interpreter::new();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Execute script file
        let filename = &args[1];
        execute_script(&mut interpreter, filename)?;
    } else {
        // Interactive mode
        run_interactive_mode(&mut interpreter)?;
    }
    Ok(())
}
