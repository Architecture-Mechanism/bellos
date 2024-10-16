# bellos
- Can be used in any Operating System but Bellande Operating System works optimally when you use Bellos Scripting Language

## Bellande Operating System Scripting Language Features
- **Command Execution**: Run both built-in and external commands.
- **Variable Assignment and Expansion**: Assign and use variables within scripts or interactive mode.
- **Control Structures**: Implement logic flow using if-else statements, while loops, and for loops.
- **Functions**: Define and call custom functions.
- **File Operations**: Perform basic file I/O operations.
- **Pipelines**: Chain commands together using pipes.
- **Input/Output Redirection**: Redirect command input and output to and from files.
- **Background Jobs**: Run commands in the background.
- **Environment Variable Handling**: Access and modify environment variables.

# Bellos Release
- https://github.com/Architecture-Mechanism/bellos/releases

# Bellos Installer
- https://github.com/Architecture-Mechanism/bellos_installer
- After install you can run the script with ```./hello_world.bellos```

# Usage of Bellande Rust Executable Builder
- https://github.com/Architecture-Mechanism/bellande_rust_executable
- ```bellande_rust_executable -d dependencies.bellande -s src -m bellos.rs -o executable/bellos``` 

# Usage of Bellande Rust Importer
- https://github.com/Architecture-Mechanism/bellande_importer

# Bellos executable run scripts
```
./bellos hello_world.bellos 
```

# Bellos interactive mode
```
./bellos
```

## Built-in Commands
### Basic Commands
- **echo [args...]**: Print arguments to standard output.
- **cd [directory]**: Change the current working directory.
- **exit**: Exit the shell.

### File Operations
- **write <filename> <content>**: Write content to a file.
- **append <filename> <content>**: Append content to a file.
- **read <filename>**: Read and display the contents of a file.
- **read_lines <filename>**: Read and display the contents of a file line by line.
- **delete <filename>**: Delete a file.

## BELLOS Usage

## Website Crates
- https://crates.io/crates/bellos

### Installation
- `cargo add bellos`

```
Name: bellos
Summary: Bellande Operating System Scripting Language
Home-page: github.com/Architecture-Mechanism/bellos
Author: Ronaldson Bellande
Author-email: ronaldsonbellande@gmail.com
License: GNU General Public License v3.0
```

## License

Bellos is a BellandeOS Scripting Language is distributed under the [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.en.html), see [LICENSE](https://github.com/Architecture-Mechanism/bellos/blob/main/LICENSE) and [NOTICE](https://github.com/Architecture-Mechanism/bellos/blob/main/LICENSE) for more information.
