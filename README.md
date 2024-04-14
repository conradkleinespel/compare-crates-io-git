# Comparing crates.io with Git repository contents

Following the attempt to introduce a backdoor into `xz`, I'm exploring ways to update Rust dependencies in a secure way.

This projects aims to detect differences between what's published on Crates.io versus what's publicly readable on Github.

## How to use


```shell
cargo run rpassword 7.1.0

# Downloading rpassword/7.1.0 to /tmp/.tmp1DSId2
# Repository is https://github.com/conradkleinespel/rpassword.git, subpath is ''
# Cloned repository to /tmp/.tmpfSg1wt
```
