# Copyright (C) 2024 Bellande Architecture Mechanism Research Innovation Center, Ronaldson Bellande

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

#!/usr/bin/env python3

import os
import shutil
import sys

def setup_bellos():
    bellos_executable = "executable/bellos"
    usr_local_bin = "/usr/local/bin/bellos"
    
    if not os.path.exists(bellos_executable):
        print(f"Error: {bellos_executable} not found.")
        return
    
    try:
        shutil.copy2(bellos_executable, usr_local_bin)
        os.chmod(usr_local_bin, 0o755)  # Make it executable
        print(f"Bellos has been copied to {usr_local_bin}")
    except IOError as e:
        print(f"Error copying file: {e}")
        return

    print("Bellos has been set up successfully.")


if __name__ == "__main__":
    if os.geteuid() != 0:
        print("This script must be run with sudo privileges.")
        sys.exit(1)
    
    setup_bellos()
