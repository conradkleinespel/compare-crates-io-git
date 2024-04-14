# Comparing crates.io with Git repository contents

Following the attempt to introduce a backdoor into `xz`, I'm exploring ways to update Rust dependencies in a secure way.

This projects aims to detect differences between what's published on Crates.io versus what's publicly readable on Github.

## How to use


```shell
cargo run rpassword 7.1.0

#Downloading rpassword/7.1.0 to /tmp/.tmpA0p9Gm
#Repository is https://github.com/conradkleinespel/rpassword.git, subpath is ''
#Cloned repository to /tmp/.tmpT0CO8T
#Default branch is master
#Sha1 announced in crates.io is 77da0606017f26e476c51d2051c6042db9c1fe4f
#Sha1 commit was 77da0606017f26e476c51d2051c6042db9c1fe4f (20/10/2022 16:06): Handle Ctrl-U in rpassword
#Sha1 commit was 77da0606017f26e476c51d2051c6042db9c1fe4f (20/10/2022 16:06): Handle Ctrl-U in rpassword

cargo run linux-raw-sys 0.4.13

#Downloading linux-raw-sys/0.4.13 to /tmp/.tmpNBXXWM
#Repository is https://github.com/sunfishcode/linux-raw-sys.git, subpath is ''
#Cloned repository to /tmp/.tmphG83ky
#Default branch is main
#Sha1 announced in crates.io is ad726d4998270502f292e1ab9a580217878b674a
#Commit not in default branch history (using revwalk)
#Commit not in default branch history (using descendants)

cargo run web-sys 0.3.67

#Downloading web-sys/0.3.67 to /tmp/.tmpbLj3NV
#Repository is https://github.com/rustwasm/wasm-bindgen.git, subpath is 'crates/web-sys'
#Cloned repository to /tmp/.tmpiYvCtH
#Default branch is main
#No sha1 announced in crates.io, crate packaged with --allow-dirty
```
