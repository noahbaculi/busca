# busca

[![CICD](https://github.com/noahbaculi/busca/actions/workflows/cicd.yml/badge.svg)](https://github.com/noahbaculi/busca/actions/workflows/cicd.yml)
[![PyPI version](https://badge.fury.io/py/busca-py.svg)](https://badge.fury.io/py/busca-py)

<img src="https://github.com/noahbaculi/busca/assets/49008873/443ead58-ff6f-4e16-982d-ba57096a6068" alt="busca logo" width="200">

CLI and library to search for files with content that most closely match the lines of a reference string.

![Busca Demo](https://github.com/noahbaculi/busca/assets/49008873/dbb40dc1-427e-4d55-839b-31e8c287bc43)

## Table of Contents

- [busca](#busca)
  - [Table of Contents](#table-of-contents)
  - [Python Library](#python-library)
  - [Command Line Interface](#command-line-interface)
    - [CLI Usage](#cli-usage)
      - [Examples](#examples)
        - [Find files that most closely match the source `file_5.py` file in a search directory](#find-files-that-most-closely-match-the-source-file_5py-file-in-a-search-directory)
        - [Find files that most closely match the source `path_to_reference.json` file in a search directory](#find-files-that-most-closely-match-the-source-path_to_referencejson-file-in-a-search-directory)
        - [Change search to scan the current working directory](#change-search-to-scan-the-current-working-directory)
        - [Narrow search to only consider `.json` files whose paths include the substring "foo" and that contain fewer than 1,000 lines](#narrow-search-to-only-consider-json-files-whose-paths-include-the-substring-foo-and-that-contain-fewer-than-1000-lines)
        - [Piped input mode to search the output of a command](#piped-input-mode-to-search-the-output-of-a-command)
    - [CLI Installation](#cli-installation)
      - [Mac OS](#mac-os)
        - [Homebrew](#homebrew)
      - [All platforms (Windows, MacOS, Linux)](#all-platforms-windows-macos-linux)
        - [Compile from source](#compile-from-source)

## Python Library

> 🐍 The Python library is renamed to `busca_py` due to a name conflict with an [existing (possibly abandoned) project](https://pypi.org/project/Busca/).

```shell
pip install busca_py
```

```python
from pathlib import Path
import busca_py as busca


reference_file_path = "./sample_dir_hello_world/file_1.py"
with open(reference_file_path, "r") as file:
    reference_string = file.read()

# Perform search with required parameters
all_file_matches = busca.search_for_lines(
    reference_string=reference_string,
    search_path="./sample_dir_hello_world",
)

# File matches are returned in descending order of percent match
closest_file_match = all_file_matches[0]
assert closest_file_match.path == Path(reference_file_path)
assert closest_file_match.percent_match == 1.0
assert closest_file_match.lines == reference_string

# Perform search for top 5 matches with additional filters
# to speed up runtime by skipping files that will not match
relevant_file_matches = busca.search_for_lines(
    reference_string=reference_string,
    search_path="./sample_dir_hello_world",
    max_lines=10_000,
    include_globs=["*.py"],
    count=5,
)

assert len(relevant_file_matches) < len(all_file_matches)

# Create new file match object
new_file_match = busca.FileMatch("file/path", 1.0, "file\ncontent")
```

## Command Line Interface

### CLI Usage

🧑‍💻️ To see usage documentation, run

```shell
busca -h
```

Output for v2.1.3

```text
Simple utility to search for files with content that most closely match the lines of a reference string

Usage: busca --ref-file-path <REF_FILE_PATH> [OPTIONS]
       <SomeCommand> | busca [OPTIONS]

Options:
  -r, --ref-file-path <REF_FILE_PATH>  Local or absolute path to the reference comparison file. Overrides any piped input
  -s, --search-path <SEARCH_PATH>      Directory or file in which to search. Defaults to CWD
  -m, --max-lines <MAX_LINES>          The number of lines to consider when comparing files. Files with more lines will be skipped [default: 10000]
  -i, --include-glob <INCLUDE_GLOB>    Globs that qualify a file for comparison
  -x, --exclude-glob <EXCLUDE_GLOB>    Globs that disqualify a file from comparison
  -c, --count <COUNT>                  Number of results to display [default: 10]
  -h, --help                           Print help
  -V, --version                        Print version
```

#### Examples

##### Find files that most closely match the source `file_5.py` file in a search directory

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

##### Find files that most closely match the source `path_to_reference.json` file in a search directory

```shell
busca --ref-file-path path_to_reference.json --search-path path_to_search_dir
```

##### Change search to scan the current working directory

```shell
busca --ref-file-path path_to_reference.json
```

##### Narrow search to only consider `.json` files whose paths include the substring "foo" and that contain fewer than 1,000 lines

```shell
busca --ref-file-path path_to_reference.json --include-glob '*.json' --include-glob '**foo**' --max-lines 1000
```

- [Glob reference](<https://en.wikipedia.org/wiki/Glob_(programming)>)

##### Piped input mode to search the output of a command

```shell
# <SomeCommand> | busca [OPTIONS]
echo 'String to find in files.' | busca
```

<details style="margin-bottom: 2em">
<summary><h5>MacOS piped input mode<h4></summary>

📝 There is an [open issue](https://github.com/crossterm-rs/crossterm/issues/396) for MacOS in [`crossterm`](https://github.com/crossterm-rs/crossterm), one of busca's dependencies, that does not allow prompt interactivity when using piped input. Therefore, when a non interactive mode is detected, the file matches will be displayed but not interactively.

This can be worked around by adding the following aliases to your shell `.bashrc` or `.zshrc` file:

> ```bash
> # Wrap commands for busca search
> busca_cmd_output() {
>     eval "$* > /tmp/busca_search.tmp" && busca -r /tmp/busca_search.tmp
> }
> ```

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

### CLI Installation

#### Mac OS

##### Homebrew

```shell
brew tap noahbaculi/busca
brew install busca
```

To update, run

```shell
brew update
brew upgrade busca
```

#### All platforms (Windows, MacOS, Linux)

##### Compile from source

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
