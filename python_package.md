# Python Package

Maintenance guide for the tooling around the Python package.

## Create and activate a virtual environment

```shell
python -m venv ./python_venv
source python_venv/bin/activate
pip install -r python_requirements.txt
```

## Document Python dependencies

```shell
pip freeze > python_requirements.txt
```

## Build the Python package locally

```shell
maturin develop --release
```

## Run Python tests

```shell
python -m unittest discover
```

## Publish the Python package to PyPI

`maturin` is deprecating its own upload command in favor of dedicated tools ([PyO3/maturin#2334](https://github.com/PyO3/maturin/issues/2334)), so the package is built with `maturin` and uploaded with `uv`.

> Note: Publishing to PyPI requires a new version number.

Build the wheel and source distribution into `target/wheels/`, then upload them with a PyPI API token:

```shell
maturin build --release --sdist
uv publish --token pypi-... target/wheels/*
```
