# busca

[![Build](https://github.com/noahbaculi/busca/actions/workflows/rust.yml/badge.svg?branch=main&event=push)](https://github.com/noahbaculi/busca/actions/workflows/rust.yml)

Simple utility to find the closest matches to a reference file in a directory based on the number of lines in the reference file that exist in each compared file.

## Compile from source

[Install Rust using `rustup`](https://www.rust-lang.org/tools/install).

Clone this repo.

In the root of this repo, run

```shell
cargo build --release
```

TODO: Add to path...

```shell
export PATH=/Users/username/bin:$PATH ???
```

To see usage documentation, run

```shell
busca -h
```
