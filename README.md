# busca

[![Build](https://github.com/noahbaculi/busca/actions/workflows/rust.yml/badge.svg?branch=main&event=push)](https://github.com/noahbaculi/busca/actions/workflows/rust.yml)

Simple utility to find the closest matches to a reference file in a directory based on the number of lines in the reference file that exist in each compared file.

## Usage

To see usage documentation, run

```shell
busca -h
```

### Demo

https://user-images.githubusercontent.com/49008873/235590754-efdeb134-feb1-44ec-bbac-44ccb737261a.mov

### MacOS

📝 There is an [open issue](https://github.com/crossterm-rs/crossterm/issues/396) for MacOS in [`crossterm`](https://github.com/crossterm-rs/crossterm), one of busca's dependencies, that does not allow prompt interactivity when using piped input. Therefore, when a non interactive mode is detected, the prompt is disabled and users are notified: `Note: Interactive prompt is not supported in this mode.`

This can be worked around by adding the following aliases to your shell `.bashrc` or `.zshrc` file:

>   ```bash
>   # Wrap commands for busca search
>   busca_cmd_output() {
>       eval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp
>   }
>   ```

One-liners to add the wrapper function:

| Shell | Command |
| --- | ---|
| Bash | `echo -e 'busca_cmd_output() {\n\teval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp\n}' >> ~/.bashrc` |
| Zsh | `echo -e 'busca_cmd_output() {\n\teval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp\n}' >> ~/.zshrc` |

Reload your shell for the function to become available:

```shell
busca_cmd_output <SomeCommand>
```

## Installation

### Homebrew

```shell
brew tap noahbaculi/busca
brew install busca
```

To update, run

```shell
brew upgrade busca
```

### Compile from source

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
