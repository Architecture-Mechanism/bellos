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

mod executor;
mod interpreter;
mod lexer;
mod parser;
mod utilities;

use crate::executor::executor::Executor;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut executor = Executor::new();

    if let Err(e) = executor.run(args) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
