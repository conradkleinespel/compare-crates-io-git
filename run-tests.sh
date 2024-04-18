#!/bin/bash

set -ex

# Everything OK
cargo run --quiet rpassword 7.1.0

# Announced commit not in history, crate is at root
cargo run --quiet linux-raw-sys 0.4.13

# Announced commit not in history, crate is in subdirectory
cargo run --quiet aes 0.8.4

# No specific commit found
cargo run --quiet web-sys 0.3.67

# Download of crate fails with a permission denied error
cargo run --quiet zeropy 0.6.6

# Crate has build script
cargo run --quiet libc 0.2.153
