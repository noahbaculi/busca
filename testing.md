# Testing

Testing guide.

## Pull comprehensive repo for testing

```shell
git clone https://github.com/Python-World/python-mini-projects.git sample-comprehensive
cd sample-comprehensive
git reset --hard e0cfd4b0fe5e0bb4d443daba594e83332d5fb720
rm -r .github
cd -
```

## Run tests

```shell
cargo test
```
