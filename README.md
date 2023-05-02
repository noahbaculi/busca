# busca

[![Build](https://github.com/noahbaculi/busca/actions/workflows/rust.yml/badge.svg?branch=main&event=push)](https://github.com/noahbaculi/busca/actions/workflows/rust.yml)

<img src="https://user-images.githubusercontent.com/49008873/235764259-078d5bfc-e4ec-441c-9178-13c3b41a1fe2.png" alt="busca logo" width="200">

Simple interactive utility to find the closest matches to a reference file in a directory based on the number of lines in the reference file that exist in each compared file.

<https://user-images.githubusercontent.com/49008873/235590754-efdeb134-feb1-44ec-bbac-44ccb737261a.mov>

## Table of Contents

- [busca](#busca)
  - [Table of Contents](#table-of-contents)
  - [Usage](#usage)
    - [Examples](#examples)
      - [Find files that most closely match the source `file_5.py` file in a search directory](#find-files-that-most-closely-match-the-source-file_5py-file-in-a-search-directory)
      - [Find files that most closely match the source `path_to_reference.json` file in a search directory](#find-files-that-most-closely-match-the-source-path_to_referencejson-file-in-a-search-directory)
      - [Change search to scan the current working directory](#change-search-to-scan-the-current-working-directory)
      - [Narrow search to only consider `.json` files whose paths include the substring "foo" and that contain fewer than 1,000 lines](#narrow-search-to-only-consider-json-files-whose-paths-include-the-substring-foo-and-that-contain-fewer-than-1000-lines)
      - [Piped input mode to search the output of a command](#piped-input-mode-to-search-the-output-of-a-command)
  - [Installation](#installation)
    - [Mac OS](#mac-os)
      - [Homebrew](#homebrew)
    - [All platforms (Windows, MacOS, Linux)](#all-platforms-windows-macos-linux)
      - [Compile from source](#compile-from-source)

## Usage

🧑‍💻️ To see usage documentation, run

```shell
busca -h
```

🛟 output for v1.3.3

```text
Simple utility to find the closest matches to a reference file or piped input based on the number of lines in the reference that exist in each compared file

Usage: busca --ref-file-path <REF_FILE_PATH> [OPTIONS]
       <SomeCommand> | busca [OPTIONS]

Options:
  -r, --ref-file-path <REF_FILE_PATH>  Local or absolute path to the reference comparison file. Overrides any piped input
  -s, --search-path <SEARCH_PATH>      Directory or file in which to search. Defaults to CWD
  -m, --max-lines <MAX_LINES>          The number of lines to consider when comparing files. Files with more lines will be skipped [default: 10000]
  -i, --include-glob <INCLUDE_GLOB>    Globs that qualify a file for comparison
  -x, --exclude-glob <EXCLUDE_GLOB>    Globs that disqualify a file from comparison
  -c, --count <COUNT>                  Number of results to display [default: 10]
      --verbose                        Print all files being considered for comparison
  -h, --help                           Print help
  -V, --version                        Print version
```

### Examples

#### Find files that most closely match the source `file_5.py` file in a search directory

```shell
❯ busca --ref-file-path sample_dir_mix/file_5.py --search-path sample_dir_mix

? Select a file to compare:  
  sample_dir_mix/file_5.py                  ++++++++++  100.0%
> sample_dir_mix/file_5v2.py                ++++++++++   97.5%
  sample_dir_mix/nested_dir/file_7.py       ++++         42.3%
  sample_dir_mix/aldras/aldras_settings.py  ++           24.1%
  sample_dir_mix/aldras/aldras_core.py      ++           21.0%
  sample_dir_mix/file_3.py                  +            13.2%
  sample_dir_mix/file_1.py                  +            11.0%
  sample_dir_mix/file_2.py                  +             9.4%
  sample_dir_mix/aldras/aldras_execute.py   +             7.5%
  sample_dir_mix/file_4.py                  +             6.9%
[↑↓ to move, enter to select, type to filter]
```

#### Find files that most closely match the source `path_to_reference.json` file in a search directory

```shell
busca --ref-file-path path_to_reference.json --search-path path_to_search_dir
```

#### Change search to scan the current working directory

```shell
busca --ref-file-path path_to_reference.json
```

#### Narrow search to only consider `.json` files whose paths include the substring "foo" and that contain fewer than 1,000 lines

```shell
busca --ref-file-path path_to_reference.json --include-glob '*.json' --include-glob '**foo**' --max-lines 1000
```

- [Glob reference](https://en.wikipedia.org/wiki/Glob_(programming))

#### Piped input mode to search the output of a command

```shell
# <SomeCommand> | busca [OPTIONS]
echo 'String to find in files.' | busca
```

<details style="margin-bottom: 2em">
<summary><h4>MacOS piped input mode<h4></summary>

📝 There is an [open issue](https://github.com/crossterm-rs/crossterm/issues/396) for MacOS in [`crossterm`](https://github.com/crossterm-rs/crossterm), one of busca's dependencies, that does not allow prompt interactivity when using piped input. Therefore, when a non interactive mode is detected, the file matches will be displayed but cannot be interactive with.

This can be worked around by adding the following aliases to your shell `.bashrc` or `.zshrc` file:

>   ```bash
>   # Wrap commands for busca search
>   busca_cmd_output() {
>       eval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp
>   }
>   ```

One-liners to add the wrapper function:

| Shell | Command                                                                                                                 |
| ----- | ----------------------------------------------------------------------------------------------------------------------- |
| Bash  | `echo -e 'busca_cmd_output() {\n\teval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp\n}' >> ~/.bashrc` |
| Zsh   | `echo -e 'busca_cmd_output() {\n\teval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp\n}' >> ~/.zshrc`  |

Reload your shell for the function to become available:

```shell
# busca_cmd_output <SomeCommand>
busca_cmd_output echo 'String to find in files.'
```

</details>

## Installation

### Mac OS

#### Homebrew

```shell
brew tap noahbaculi/busca
brew install busca
```

To update, run

```shell
brew update
brew upgrade busca
```

### All platforms (Windows, MacOS, Linux)

#### Compile from source

0. Install Rust [using `rustup`](https://www.rust-lang.org/tools/install).

1. Clone this repo.

2. In the root of this repo, run

    ```shell
    cargo build --release
    ```

3. Add to path. For example, by copying the compiled binary to your local bin directory.

    ```shell
    cp target/release/busca $HOME/bin/
    ```
