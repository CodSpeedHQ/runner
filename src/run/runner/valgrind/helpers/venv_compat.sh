#!/usr/bin/env bash

function add_symlink() {
    local venv_python="$1"

    system_python="$(readlink -f "$venv_python")"
    if [ -z "$system_python" ]; then
        echo "Error: Failed to resolve real path for $venv_python" >&2
        return 1
    fi

    system_path="$(dirname $(dirname "$system_python"))"
    venv_path="$(dirname $(dirname "$venv_python"))"
    echo "Python installation (system): $system_path"
    echo "Python installation (venv): $venv_path"

    libpython_name="$(ldd "$venv_python" 2>/dev/null | grep -o -m1 'libpython[^[:space:]]*' || true)"
    if [ -z "$libpython_name" ]; then
        echo "Error: exact libpython name not found in $(ldd $venv_python)" >&2
        return 1
    fi
    echo "Found linked libpython: $libpython_name"

    # Create the symlink in the virtual environment
    venv_link="$venv_path/lib/$libpython_name"
    system_link="$system_path/lib/$libpython_name"
    if [ -e "$venv_link" ]; then
        echo "Symlink already exists: $venv_link"
    else
        echo "Creating symlink: $system_link -> $venv_link"
        ln -s "$system_link" "$venv_link"
        if [ $? -ne 0 ]; then
            echo "Error: Failed to create symlink $venv_link" >&2
            return 1
        fi
    fi
}

uv_python="$(uv python find 2>/dev/null || true)"
if [ -n "$uv_python" ]; then
    add_symlink "$uv_python"
    if [ $? -ne 0 ]; then
        echo "Error: Failed to add symlink for uv venv" >&2
        exit 1
    fi
else
    echo "Didn't find uv venv, continuing..."
fi

python3_path="$(which python3 2>/dev/null || true)"
if [ -n "$python3_path" ]; then
    echo "Found system Python: $python3_path"

    if ldd "$python3_path" | grep -q "libpython.*not found"; then
        add_symlink "$python3_path"
        if [ $? -ne 0 ]; then
            echo "Error: Failed to add symlink for system Python" >&2
            exit 1
        fi
    else
        echo "System python is already correctly linked, continuing..."
    fi
fi
