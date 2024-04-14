# Comparing crates.io with Git repository contents

Following the attempt to introduce a backdoor into `xz`, I'm exploring ways to update Rust dependencies in a secure way. This projects aims to surface differences between what is in public git repositories versus what is published on crates.io for further analysis.

## What it does

- Warns if the commit used to package the crate is not present in the default branch on the public git repository;
- Warns if the crate was built with the `--allow-dirty` option;
- Shows file differences between the archive distributed via crates.io and the contents of the public git repository, trying to minimize false positives where possible.

## How to use


```shell
cargo run rpassword 7.1.0

#Downloading rpassword/7.1.0 to /tmp/.tmpipLlsn
#Repository is https://github.com/conradkleinespel/rpassword.git, subpath is ''
#Cloned repository to /tmp/.tmp5yLXfZ
#Default branch is master
#Sha1 announced in crates.io is 77da0606017f26e476c51d2051c6042db9c1fe4f
#Sha1 commit was 77da0606017f26e476c51d2051c6042db9c1fe4f (20/10/2022 16:06): Handle Ctrl-U in rpassword
#Sha1 commit was 77da0606017f26e476c51d2051c6042db9c1fe4f (20/10/2022 16:06): Handle Ctrl-U in rpassword
#Commit is in history, checking it out
#Diffing /tmp/.tmpipLlsn/rpassword-7.1.0 and /tmp/.tmp5yLXfZ/
#Files /tmp/.tmpipLlsn/rpassword-7.1.0/Cargo.toml and /tmp/.tmp5yLXfZ/Cargo.toml differ


cargo run linux-raw-sys 0.4.13

#Downloading linux-raw-sys/0.4.13 to /tmp/.tmpEEQBj8
#Repository is https://github.com/sunfishcode/linux-raw-sys.git, subpath is ''
#Cloned repository to /tmp/.tmppYFcPu
#Default branch is main
#Sha1 announced in crates.io is ad726d4998270502f292e1ab9a580217878b674a
#Commit not in default branch history (using revwalk)
#Commit not in default branch history (using descendants)
#Diffing /tmp/.tmpEEQBj8/linux-raw-sys-0.4.13 and /tmp/.tmppYFcPu/
#Files /tmp/.tmpEEQBj8/linux-raw-sys-0.4.13/Cargo.toml and /tmp/.tmppYFcPu/Cargo.toml differ
#Files /tmp/.tmpEEQBj8/linux-raw-sys-0.4.13/src/aarch64/general.rs and /tmp/.tmppYFcPu/src/aarch64/general.rs differ
#...

cargo run web-sys 0.3.67

#Downloading web-sys/0.3.67 to /tmp/.tmpcrjvBa
#Repository is https://github.com/rustwasm/wasm-bindgen.git, subpath is 'crates/web-sys'
#Cloned repository to /tmp/.tmpYeyJ5e
#Default branch is main
#No sha1 announced in crates.io, crate packaged with --allow-dirty
#Diffing /tmp/.tmpcrjvBa/web-sys-0.3.67 and /tmp/.tmpYeyJ5e/crates/web-sys
#Files /tmp/.tmpcrjvBa/web-sys-0.3.67/Cargo.toml and /tmp/.tmpYeyJ5e/crates/web-sys/Cargo.toml differ
#Files /tmp/.tmpcrjvBa/web-sys-0.3.67/src/features/gen_AddEventListenerOptions.rs and /tmp/.tmpYeyJ5e/crates/web-sys/src/features/gen_AddEventListenerOptions.rs differ
#...
```
