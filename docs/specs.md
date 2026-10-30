# Writing specs

A spec tells compl what can follow a command: its subcommands, its options and what kind of argument it takes. Specs are JSON files in [`specs/`](../specs), compiled into the binary.

## Format

Each file holds an array of specs. A spec has these fields; only `name` is required:

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | string | The command or subcommand |
| `aliases` | array of strings | Other names that mean the same thing |
| `description` | string | Shown next to the suggestion in the list |
| `args` | string | What its arguments are; see below |
| `subcommands` | array of specs | Subcommands, each a spec of its own |
| `options` | array of `{ "name", "description" }` | Options, suggested once a word starts with `-` |

`args` is one of:

| Value | Suggests |
| --- | --- |
| `path` | Files and directories |
| `directory` | Directories only |
| `branch` | Git branches, local and remote |
| `remote` | Git remotes |
| `script` | Scripts from the nearest `package.json` |
| `target` | Targets from the Makefile in the current directory |

Leave `args` out when the arguments are free text, such as a commit message, so no list shows for them.

## Example

```json
[
  {
    "name": "just",
    "description": "Run project recipes",
    "options": [
      { "name": "--list", "description": "List recipes" }
    ],
    "subcommands": [
      { "name": "fmt", "description": "Format the justfile" }
    ],
    "args": "path"
  }
]
```

Names match without regard to case, and a command typed as a path (`/usr/bin/git`) or with a Windows extension (`git.exe`) finds the same spec.

## Adding a spec

1. Add your spec to the file in `specs/` that fits it, or create a new file.
2. If you created a file, add it to the `SOURCES` list in [`src/complete/specs.rs`](../src/complete/specs.rs).
3. Run `cargo test`; the tests check that every spec parses.
4. Try it: `cargo run -- complete "just f"`.

Keep descriptions short (a few words, no trailing period), and list what people use day to day rather than every option a command has.
