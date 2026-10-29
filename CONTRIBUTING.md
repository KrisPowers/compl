# Contributing

Thanks for helping. Bug reports, new and improved [specs](docs/specs.md), and fixes are all welcome.

## Getting set up

You need a stable [Rust toolchain](https://rustup.rs).

```bash
git clone https://github.com/KrisPowers/autocomplete
cd autocomplete
cargo test
cargo run -- run
```

`cargo run -- run` starts your shell inside a development build. `cargo run -- complete "git ch"` prints the suggestions for a line without starting a session, which is the quickest way to check completion changes.

## Layout

| Path | What lives there |
| --- | --- |
| `src/session.rs` | The PTY session: input, output and key handling |
| `src/screen.rs` | Reading the command line off the screen model |
| `src/overlay.rs` | Drawing and clearing suggestions |
| `src/markers.rs` | Pulling prompt markers out of the shell's output |
| `src/complete/` | Parsing the line and working out suggestions |
| `src/shell.rs` | Starting each shell, `init` and `setup` |
| `shell/` | The scripts loaded into each shell |
| `specs/` | Command specs |
| `scripts/` | Installers and release helpers |
| `docs/` | Documentation |

[How it works](docs/how-it-works.md) explains how these fit together.

## Before opening a pull request

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks on Linux, macOS and Windows, plus syntax checks for the shell scripts.

- Keep each pull request to one change, and describe how you tested it, including which shells and terminals you tried.
- Commit messages follow `type(scope): summary`, for example `feat(specs): add just`.
- If you change behavior, update the docs in the same pull request.

## Releasing

Maintainers publish a GitHub release with a `vX.Y.Z` tag after bumping the version in `Cargo.toml` and adding a `CHANGELOG.md` entry. The release workflow builds and attaches the archives and opens a pull request updating the Homebrew formula.

## Conduct

Everyone taking part is expected to follow the [code of conduct](CODE_OF_CONDUCT.md).
