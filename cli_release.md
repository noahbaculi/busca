# CLI Release

Guide for cutting a CLI release.

## Pick a version number

> [Methodology: X.Y.Z, which corresponds to major.minor.patch.](https://semver.org/)

Update the version in the `Cargo.toml` file.

```toml
[package]
name = "busca"
version = "3.0.0"
...
```

## Build a universal binary for macOS ARM and x86

```shell
# macOS ARM architecture
rustup target install aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
file target/aarch64-apple-darwin/release/busca  # --> Mach-O 64-bit executable arm64

# macOS x86/Intel architecture
rustup target install x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
file target/x86_64-apple-darwin/release/busca   # --> Mach-O 64-bit executable x86_64

# Build universal binary
mkdir -p target/apple-darwin-universal/release
lipo -create target/x86_64-apple-darwin/release/busca target/aarch64-apple-darwin/release/busca -output target/apple-darwin-universal/release/busca
file target/apple-darwin-universal/release/busca   # --> Mach-O universal binary with 2 architectures: [x86_64:Mach-O 64-bit executable x86_64] [arm64]

# Copy binary to local $PATH for development
cp target/apple-darwin-universal/release/busca python_venv/bin/busca
```

## Create the TAR archive and GitHub release

```shell
cd target/apple-darwin-universal/release/ 
tar -czf busca-mac.tar.gz busca
shasum -a 256 busca-mac.tar.gz   # --> __sha_for_tar__
cd -
```

Add GitHub release with the version number and release notes. Upload the generated `busca-mac.tar.gz` file in the `target/apple-darwin-universal/release/` directory.
Once published, copy the URL of the TAR archive for later use with the Homebrew Tap (`__link_to_tar_in_the_github_release__`).

> ex: <https://github.com/noahbaculi/busca/releases/download/v3.0.0/busca-mac.tar.gz>

## Update the Homebrew tap

[Add Homebrew version](https://github.com/noahbaculi/homebrew-busca).

## Demo recording tips

- Use macOS built-in screen recording to capture screen.
- Use Oh-My-Posh `Bubbles` terminal theme.
- Use iTerm2's `Advanced Paste` to simulate typing effect. `Edit > Paste Special > Advanced Paste...`
