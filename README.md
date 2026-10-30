# autocomplete

Inline, as-you-type suggestions for commands and paths, in every terminal and shell.

Type `cd` and a space, and a list of directories appears under your cursor; type `Do` and it narrows to the ones starting with it. Its first row is what you typed. Press Tab to move down the list; each directory you land on goes onto the command line. Keep typing to narrow the list, and press Enter to run the line as it stands.

```text
~/code $ cd Documents/
            Do
            Documents
            Downloads
            Dotfiles
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

Start typing. When suggestions exist, a list shows under the cursor:

| Key | What it does |
| --- | --- |
| Tab | Move down the list and put that suggestion on the command line |
| Shift+Tab | Move up the list; the first row puts back what you typed |
| Up, Down | Move up or down the list, the same as Shift+Tab and Tab |
| Enter | Run the command line as it stands |
| Esc | Put back what you typed and hide the list until you type again |

When no list is showing, every key behaves exactly as it does without autocomplete, so Tab still reaches your shell's own completion and Up and Down still go through history.

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
