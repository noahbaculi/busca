# CLI Release

CLI releasing guide.

## Pick Version Number

> [Methodology: X.Y.Z, which corresponds to major.minor.patch.](https://semver.org/)

Update the version in the `Cargo.toml` file.

```toml
[package]
name = "busca"
version = "0.1.1"
...
```

## Build Universal Binary for MacOS ARM and x86

```shell
# MacOS ARM architecture
rustup target install aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
file target/aarch64-apple-darwin/release/busca  # --> Mach-O 64-bit executable arm64

# MacOS x86/Intel architecture
rustup target install x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin 
file target/x86_64-apple-darwin/release/busca   # --> Mach-O 64-bit executable x86_64

# Build universal binary
mkdir -p target/apple-darwin-universal/release
lipo -create target/x86_64-apple-darwin/release/busca target/aarch64-apple-darwin/release/busca -output target/apple-darwin-universal/release/busca
file target/apple-darwin-universal/release/busca   # --> Mach-O universal binary with 2 architectures: [x86_64:Mach-O 64-bit executable x86_64] [arm64]
```

## Create TAR archive and GitHub release

```shell
cd target/apple-darwin-universal/release/ 
tar -czf busca-mac.tar.gz busca
shasum -a 256 busca-mac.tar.gz   # --> __sha_for_tar__
cd -
```

Add GitHub release with the version number and release notes. Copy the URL of the TAR archive for later use (`__link_to_tar_in_the_github_release__`).

> ex: <https://github.com/noahbaculi/busca/releases/download/v0.1.1/busca-mac.tar.gz>

## Update the Homebrew Tap

[Add Homebrew version](https://github.com/noahbaculi/homebrew-busca).
