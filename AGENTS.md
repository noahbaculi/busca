# Agent notes

Read `CONTEXT.md` for the domain vocabulary and `docs/adr/` for settled decisions before changing search behavior or the public surface.

## Releases

Busca ships through three manual channels, each public and hard to take back: a GitHub release with the universal macOS binary (`cli_release.md`), the Homebrew formula in the separate [homebrew-busca](https://github.com/noahbaculi/homebrew-busca) tap, and the `busca_py` package on PyPI (`python_package.md`). Bump versions, tag, create releases, upload to PyPI and edit the tap only when the user asks. PyPI needs a fresh version number for every upload.
