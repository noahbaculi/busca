# busca

[![CICD](https://github.com/noahbaculi/busca/actions/workflows/cicd.yml/badge.svg)](https://github.com/noahbaculi/busca/actions/workflows/cicd.yml)
[![PyPI version](https://badge.fury.io/py/busca-py.svg)](https://badge.fury.io/py/busca-py)

See [`CHANGELOG.md`](./CHANGELOG.md) for release history.

<img src="https://github.com/noahbaculi/busca/assets/49008873/443ead58-ff6f-4e16-982d-ba57096a6068" alt="busca logo" width="200">

CLI and library to search for files with content that most closely match the lines of a reference string.

![Animated demo: busca ranks files by similarity and shows a colored line-diff](docs/demo.gif)

## Table of contents

- [busca](#busca)
  - [Table of contents](#table-of-contents)
  - [Python library](#python-library)
  - [Command line interface](#command-line-interface)
    - [CLI usage](#cli-usage)
      - [Examples](#examples)
        - [Find files that most closely match the source `file_5.py` file in a search directory](#find-files-that-most-closely-match-the-source-file_5py-file-in-a-search-directory)
        - [Find files that most closely match the source `path_to_reference.json` file in a search directory](#find-files-that-most-closely-match-the-source-path_to_referencejson-file-in-a-search-directory)
        - [Change search to scan the current working directory](#change-search-to-scan-the-current-working-directory)
        - [Narrow search to only consider `.json` files whose paths match the glob `**/*foo*` and that contain fewer than 1,000 lines](#narrow-search-to-only-consider-json-files-whose-paths-match-the-glob-foo-and-that-contain-fewer-than-1000-lines)
        - [Piped input mode to search the output of a command](#piped-input-mode-to-search-the-output-of-a-command)
  - [Versioning](#versioning)
    - [CLI installation](#cli-installation)
      - [macOS](#macos)
        - [Homebrew](#homebrew)
      - [All platforms (Windows, macOS, Linux)](#all-platforms-windows-macos-linux)
        - [Compile from source](#compile-from-source)

## Python library

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

# Perform a search with required parameters
all_file_comparisons = busca.search(
    reference_string=reference_string,
    search_path="./sample_dir_hello_world",
)

# Comparisons are returned in descending order of similarity_ratio
closest_file_comparison = all_file_comparisons[0]
assert closest_file_comparison.path == Path(reference_file_path)
assert closest_file_comparison.similarity_ratio == 1.0
assert closest_file_comparison.content == reference_string

# Perform a search for the top 5 comparisons with additional filters
# to speed up runtime by skipping files that will not match
relevant_file_comparisons = busca.search(
    reference_string=reference_string,
    search_path="./sample_dir_hello_world",
    max_file_lines=10_000,
    include_glob=["*.py"],
    count=5,
)

assert len(relevant_file_comparisons) < len(all_file_comparisons)

# Perform a search that drops candidates below a similarity floor
strong_file_comparisons = busca.search(
    reference_string=reference_string,
    search_path="./sample_dir_hello_world",
    include_glob=["*.py"],
    min_similarity_ratio=0.5,
)

assert all(fc.similarity_ratio >= 0.5 for fc in strong_file_comparisons)

# Create a new FileComparison object
new_file_comparison = busca.FileComparison("file/path", 1.0, "file\ncontent")
```

## Command line interface

### CLI usage

🧑‍💻️ To see usage documentation, run

```shell
busca -h
```

Output for v3.0.0

```text
Simple utility to search for files with content that most closely match the lines of a reference string

Usage: busca --ref-file-path <REF_FILE_PATH> [OPTIONS]
       <SomeCommand> | busca [OPTIONS]

Options:
  -r, --ref-file-path <REF_FILE_PATH>
          Local or absolute path to the reference comparison file. Overrides any piped input
  -s, --search-path <SEARCH_PATH>
          Directory or file in which to search. Defaults to CWD
  -m, --max-file-lines <MAX_FILE_LINES>
          The maximum number of lines a candidate file may have. Candidates with more lines (or zero lines) are skipped entirely [default: 10000]
  -i, --include-glob <INCLUDE_GLOB>
          Globs that qualify a file for comparison
  -x, --exclude-glob <EXCLUDE_GLOB>
          Globs that disqualify a file from comparison
  -c, --count <COUNT>
          Number of results to display [default: 10]
      --min-similarity-ratio <MIN_SIMILARITY_RATIO>
          Drop comparisons whose similarity ratio is below this value (in [0.0, 1.0]). Applied during the search, before the --count limit
      --format <FORMAT>
          Output format for the ranked results [default: human] [possible values: human, json]
      --with-content
          Include each file's content in JSON output. Ignored for the human format
      --no-interactive
          Print the ranked list instead of launching the interactive picker
  -h, --help
          Print help
  -V, --version
          Print version
```

#### Examples

##### Find files that most closely match the source `file_5.py` file in a search directory

```shell
❯ busca --ref-file-path sample_dir_mix/file_5.py --search-path sample_dir_mix

? Select a file to compare:  
  sample_dir_mix/file_5.py                  ++++++++++  100.0%
> sample_dir_mix/file_5v2.py                ++++++++++   97.2%
  sample_dir_mix/nested_dir/file_7.py       +++++        45.8%
  sample_dir_mix/aldras/aldras_core.py      ++           21.7%
  sample_dir_mix/aldras/aldras_settings.py  ++           21.2%
  sample_dir_mix/file_3.py                  ++           16.8%
  sample_dir_mix/file_1.py                  +            14.1%
  sample_dir_mix/file_2.py                  +            13.7%
  sample_dir_mix/aldras/aldras_execute.py   +            11.9%
  sample_dir_mix/file_4.py                  +             9.0%
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

##### Narrow search to only consider `.json` files whose paths match the glob `**/*foo*` and that contain fewer than 1,000 lines

```shell
busca --ref-file-path path_to_reference.json --include-glob '*.json' --include-glob '**/*foo*' --max-file-lines 1000
```

- [Glob reference](<https://en.wikipedia.org/wiki/Glob_(programming)>)

##### Piped input mode to search the output of a command

```shell
# <SomeCommand> | busca [OPTIONS]
echo 'String to find in files.' | busca
```

<details style="margin-bottom: 2em">
<summary><h5>macOS piped input mode</h5></summary>

📝 [`crossterm`](https://github.com/crossterm-rs/crossterm), one of busca's dependencies, has an [open issue](https://github.com/crossterm-rs/crossterm/issues/396) on macOS that blocks prompt interactivity with piped input. When busca detects a non-interactive mode, it prints the file comparisons without the interactive picker.

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

##### Structured output for scripts

```shell
busca --ref-file-path path_to_reference.py --include-glob '*.py' --format json
```

```json
[
  { "path": "src/file_5.py", "similarity_ratio": 1.0 },
  { "path": "src/file_5v2.py", "similarity_ratio": 0.9722 }
]
```

Add `--with-content` to include each file's body. `--format json` is always
non-interactive; for the human grid without the picker, use `--no-interactive`.

busca uses these exit codes so scripts can branch on the result:

| Exit code | Meaning |
| --------- | ------- |
| `0` | At least one comparison survived `--min-similarity-ratio` and `--count` |
| `1` | No comparisons matched |
| `2` | An error occurred (bad glob, missing search path, unreadable reference) |

On an empty result busca writes nothing to stdout, prints `No files found` to
stderr, and exits `1`, so scripts should branch on the exit code rather than
parse stdout for an empty array.

## Versioning

- **Rust MSRV**: 1.85 (enforced via `Cargo.toml` `rust-version`).
- **Python**: 3.11 or later.
- **Semver**: breaking changes ship on major version bumps. The Rust public surface covered by semver is `Args`, `FileComparison`, `Error`, `run_search`, `run_search_with_progress`, `get_similarity_ratio`, and `format_file_comparisons`. Items not in this list are implementation details and may change in any release.
- **Python public surface**: `busca_py.search` and `busca_py.FileComparison` as declared in `busca_py.pyi`.

### Migrating from 2.x to 3.x

Python callers should rename the kwargs and the result type:

```python
# 2.x
results = busca_py.search_for_lines(
    reference_string=ref,
    search_path="./src",
    max_lines=10_000,
    include_globs=["*.py"],
    exclude_globs=["*.yml"],
)
top = results[0]
top.percent_match  # float
top.lines          # str

# 3.x
results = busca_py.search(
    reference_string=ref,
    search_path="./src",
    max_file_lines=10_000,
    include_glob=["*.py"],   # also accepts a bare string
    exclude_glob=["*.yml"],
)
top = results[0]
top.similarity_ratio  # float, see ADR-0001 for the metric change
top.content           # str
```

See [`CHANGELOG.md`](./CHANGELOG.md) for the full rename table and the Rust-side migration notes.

### CLI installation

#### macOS

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

#### All platforms (Windows, macOS, Linux)

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
