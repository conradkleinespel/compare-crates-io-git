# Comparing crates.io with Git repository contents

Following the attempt to introduce a backdoor into `xz`, I'm exploring ways to update Rust dependencies in a secure way. This projects aims to surface differences between what is in public git repositories versus what is published on crates.io for further analysis.

## What it does

- Warns if the commit used to package the crate is not present in the default branch on the public git repository;
- Warns if the crate was built with the `--allow-dirty` option;
- Shows file differences between the archive distributed via crates.io and the contents of the public git repository, trying to minimize false positives where possible;
- Warns if the crate uses a non-standard build script, which could be used to hide build-steps from tools looking only at `build.rs`;
- Warns if the crate on crates.io contains binary files, which could be used to hide things.

## How to use

```shell
cargo run rpassword 7.1.0
```

See [run-tests.sh](run-tests.sh) for more examples.
