# bellos

## Bellande Operating System Scripting Language written in Rust
- Variable Assignment
- Command Execution
- I/O Redirection
- Interactive Mode and File Execution
- Error handling
- Control structures
- Functions
- Built-in commands
- Environment variables
- redirection support


# Usage of Bellande Rust Importer
- https://github.com/Architecture-Mechanism/bellande_importer

# Bellos executable run scripts
```
./bellos hello_world.bellos 
```

## BELLOS Usage
```
#!/usr/bin/env bellos
# File: hello_world.bellos

# Simple Hello World script
echo "Hello, World!"

# Using variables
name="Bellos"
echo "Welcome to $name programming!"
```

``` 
#!/usr/bin/env bellos
# File: basic_math.bellos

# Demonstrating arithmetic operations
a=5
b=3

sum=$((a + b))
difference=$((a - b))
product=$((a * b))
quotient=$((a / b))

echo "Sum: $sum"
echo "Difference: $difference"
echo "Product: $product"
echo "Quotient: $quotient"
```

```
#!/usr/bin/env bellos
# File: control_structures.bellos

# Demonstrating if statements and loops

# If statement
if [ $# -eq 0 ]
then
    echo "No arguments provided"
elif [ $# -eq 1 ]
then
    echo "One argument provided: $1"
else
    echo "Multiple arguments provided"
fi

# For loop
echo "Counting from 1 to 5:"
for i in 1 2 3 4 5
do
    echo $i
done

# While loop
echo "Countdown:"
count=5
while [ $count -gt 0 ]
do
    echo $count
    count=$((count - 1))
done
```

```
#!/usr/bin/env bellos
# File: functions.bellos

# Defining and using functions

function greet() {
    echo "Hello, $1!"
}

function add() {
    echo $(($1 + $2))
}

# Calling functions
greet "User"
result=$(add 3 4)
echo "3 + 4 = $result"
```


```
#!/usr/bin/env bellos
# File: file_operations.bellos

# Demonstrating file operations

# Writing to a file
echo "This is a test file" > test.txt
echo "Adding another line" >> test.txt

# Reading from a file
echo "Contents of test.txt:"
cat test.txt

# Using a while loop to read file line by line
echo "Reading file line by line:"
while read -r line
do
    echo "Line: $line"
done < test.txt

# Cleaning up
rm test.txt
```

```
#!/usr/bin/env bellos
# File: string_manipulation.bellos

# Demonstrating string manipulation

string="Hello, Bellos!"

# String length
echo "Length of string: ${#string}"

# Substring
echo "First 5 characters: ${string:0:5}"

# String replacement
new_string=${string/Bellos/World}
echo "Replaced string: $new_string"

# Converting to uppercase
echo "Uppercase: ${string^^}"

# Converting to lowercase
echo "Lowercase: ${string,,}"
```

## Website Crates
- https://crates.io/crates/bellos

### Installation
- `cargo add bellos`

```
Name: bellos
Version: 0.0.1
Summary: Bellande Operating System Scripting Programming Language
Home-page: github.com/RonaldsonBellande/bellos
Author: Ronaldson Bellande
Author-email: ronaldsonbellande@gmail.com
License: GNU General Public License v3.0
```

## License

BellandeOS Scripting Language is distributed under the [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.en.html), see [LICENSE](https://github.com/Architecture-Mechanism/bellos/blob/main/LICENSE) and [NOTICE](https://github.com/Architecture-Mechanism/bellos/blob/main/LICENSE) for more information.
