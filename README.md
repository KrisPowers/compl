# autocomplete

Inline, as-you-type suggestions for commands and paths, in every terminal and shell.

Type `cd` and a space, and the first matching directory appears after your cursor, dimmed. Press Tab to open the full list and move through it, keep typing to narrow it, and press Enter to put your pick on the command line. Nothing runs until you press Enter again.

```text
~/code $ cd Do|cuments/
              Documents/
              Downloads/
              Dotfiles/
```

It works the same way for command names, subcommands and options (`git che` suggests `checkout`), branches, remotes, package.json scripts, Makefile targets and files.

- Works in any terminal: macOS Terminal, iTerm2, Windows Terminal, the Windows console, GNOME Terminal, Konsole, Alacritty, kitty, WezTerm, VS Code and others.
- Works with bash, zsh, fish, PowerShell 7, Windows PowerShell and cmd.
- Suggestions are sorted alphabetically, with digits and symbols ahead of letters.
- Your shell config, prompt theme and key bindings stay as they are.

## Installation

### macOS and Linux

```bash
curl -fsSL https://raw.githubusercontent.com/KrisPowers/autocomplete/main/scripts/install.sh | sh
```

This installs the binary into `~/.local/bin` and adds autocomplete to your shell's startup file.

### Windows

In PowerShell:

```powershell
irm https://raw.githubusercontent.com/KrisPowers/autocomplete/main/scripts/install.ps1 | iex
```

This installs `autocomplete.exe` into `%LOCALAPPDATA%\Programs\autocomplete`, adds it to your PATH and adds autocomplete to your PowerShell profile.

### Homebrew

```bash
brew tap KrisPowers/autocomplete https://github.com/KrisPowers/autocomplete
brew install autocomplete
autocomplete setup
```

Open a new terminal once installed. The installers accept a few environment variables, such as a pinned version; see [installation options](docs/installation.md).

## Usage

Start typing. When a suggestion shows:

| Key | What it does |
| --- | --- |
| Tab | Open the list, then move to the next suggestion |
| Shift+Tab | Move to the previous suggestion |
| Up, Down | Move through the open list |
| Enter | Put the selected suggestion on the command line |
| Enter again | Run the command |
| Esc | Hide suggestions until you type again |

When what you typed already matches the suggestion, Enter runs the command straight away. When no suggestion is showing, every key behaves exactly as it does without autocomplete, so Tab still reaches your shell's own completion.

To try autocomplete without changing any startup file, run it directly:

```bash
autocomplete run --shell zsh
```

Exit the shell as usual (`exit` or Ctrl+D) to leave it.

### Commands

| Command | What it does |
| --- | --- |
| `autocomplete` | Start your shell with suggestions turned on |
| `autocomplete run --shell <shell>` | Start a specific shell: `bash`, `zsh`, `fish`, `pwsh`, `powershell` or `cmd` |
| `autocomplete setup [shell]` | Add autocomplete to the shell's startup file |
| `autocomplete setup [shell] --remove` | Take it out again |
| `autocomplete init <shell>` | Print the snippet the startup file evaluates |
| `autocomplete complete "<line>"` | Print the suggestions for a command line |

## Configuration

| Variable | Effect |
| --- | --- |
| `AUTOCOMPLETE_DISABLE=1` | Start shells without autocomplete, even with the startup line in place |

Suggestions for a command come from a small JSON spec. Adding or improving one is the easiest way to contribute; see [writing specs](docs/specs.md).

## Documentation

- [How it works](docs/how-it-works.md)
- [Shells and terminals](docs/shells.md)
- [Installation options](docs/installation.md)
- [Writing specs](docs/specs.md)
- [Troubleshooting](docs/troubleshooting.md)

## Contributing

Bug reports, specs and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT, see [LICENSE](LICENSE).
