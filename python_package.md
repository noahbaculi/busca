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

First, add the `MATURIN_USERNAME` and `MATURIN_PASSWORD` environment variables using the values of an API token from PyPI.

> Note: Publishing to PyPI requires a new version number.

```shell
maturin publish
```
