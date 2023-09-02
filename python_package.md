# Python Package

Maintenance guide for the tooling around the Python package.

## Create and Activate Virtual Environment

```shell
python -m venv ./python_venv
source python_venv/bin/activate
pip install -r python_requirements.txt
```

## Document Python dependencies

```shell
pip freeze > python_requirements.txt
```

## Build Python package locally

```shell
maturin develop --release
```

## Run Python tests

```shell
python -m unittest discover
```

## Publish Python package to PyPi

First, add the `MATURIN_USERNAME` and `MATURIN_PASSWORD` environment variables using the values of an API token from PyPI.

> Note: Publishing to PyPI requires a new version number.

```shell
maturin publish
```
