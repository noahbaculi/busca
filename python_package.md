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

## Build Python package

```shell
maturin develop --release
```
