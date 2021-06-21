Windows Wave API in Rust
========================
This is a **hobby** project. The goal is to learn Rust and a bit about making sounds.

Requirements
------------
* windows
* rustc

Usage
-----
Just use `cargo build`.

Debugging with VSCode & rust-analyser
-------------------------------------
The following tasks need to be in your ```tasks.json``` file : 

    {
        "type": "cargo",
        "label": "sine_wave",
        "command": "build",
        "args": [
            "--bin",
            "sine_wave"
        ],
        "problemMatcher": [
            "$rustc"
        ],
        "group": "build",
        "options": {
            "cwd": "${workspaceFolder}"
        }
    }

The following configuration needs to be in your ```launch.json``` file :

    {
        "name": "sine_wave",
        "type": "cppvsdbg",
        "request": "launch",
        "program": "${workspaceFolder}/target/debug/sine_wave.exe",
        "args": [],
        "stopAtEntry": false,
        "cwd": "${workspaceFolder}",
        "environment": [],
        "console": "newExternalWindow",
        "preLaunchTask": "sine_wave"
    }
