# Testing

Testing guide.

## Pull the benchmark fixture

The `run_search` benchmark searches the Django package source, pinned to release
tag 5.2.15 for reproducibility.

```shell
git clone --depth 1 --branch 5.2.15 https://github.com/django/django.git sample-comprehensive
```

## Sample run to time

```shell
cargo run --release -- -r sample-comprehensive/django/forms/models.py -s sample-comprehensive/django
```

## Run tests

```shell
cargo test
```
