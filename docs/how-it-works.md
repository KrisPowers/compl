# How it works

Every shell edits its command line differently, and every terminal draws it differently, but all of them draw the command line on screen. compl works from what is on screen, which is why one binary covers every shell and terminal.

## The session

`compl run` opens a pseudo terminal (a PTY on macOS and Linux, ConPTY on Windows) and starts your shell inside it. From then on:

1. Every key you press goes to the shell unchanged, except the few keys that act on a visible suggestion.
2. Everything the shell prints goes to your terminal unchanged, and also into a terminal emulator held in memory (the [vt100](https://crates.io/crates/vt100) crate). That screen model always knows what your terminal is showing.
3. When the shell is waiting for input, compl reads the command line back off the screen model, works out the suggestions and draws them on top of your terminal.

Before the shell's next output reaches the terminal, compl redraws the rows it covered from the screen model, so suggestions never leave marks behind and the shell never sees them.

## Prompt markers

To know where the command line starts, compl loads a small integration script into the shell after your own startup files. The script makes the prompt print three invisible markers:

| Marker | Meaning |
| --- | --- |
| `ESC ] 6973 ; A ; <directory> BEL` | A prompt is about to be drawn, in this directory |
| `ESC ] 6973 ; P ; <PATH> BEL` | The PATH the shell finds commands on |
| `ESC ] 6973 ; B BEL` | The prompt is drawn; input starts at the cursor |

compl removes the markers from the output, so your terminal never sees them. When `B` arrives, the cursor position is the start of the input. Anything between there and the cursor is what you typed, however the shell drew it: history recall, the shell's own completion and cursor movement all show up correctly.

Suggestions only show when the cursor is at the end of the input on the prompt line. When a program is running, the screen is in full-screen mode, or the cursor moved back into the line, compl stays out of the way.

## Suggestions

The command line is split into words using the shell's own quoting rules (POSIX for bash, zsh and fish, PowerShell's for PowerShell, cmd's for cmd). Then:

- The first word of a command, after `sudo`, `env`, variable assignments and the like, is completed from the programs on PATH and the shell's built-in commands.
- Later words are completed from the command's [spec](specs.md): its subcommands, its options, and its arguments, which can be paths, directories, git branches, git remotes, package.json scripts or Makefile targets.
- Paths are only suggested where a command's spec says it takes them, so a word that is not a known command gets nothing.
- A suggestion that is exactly what you already typed is left out.

Matching ignores case. Suggestions are sorted alphabetically, ignoring case, with digits and symbols ahead of letters, and a word sorts ahead of longer words that start with it. Each suggestion is quoted for your shell before it is typed in, so a directory called `My Music` becomes `My\ Music/` in bash and `'My Music\` in PowerShell.

## Picking a suggestion

The list's first row is what you typed; the suggestions follow it. Tab moves the highlight down and types the highlighted suggestion into the shell in place of the word you were typing, as if you had typed it: only the part that differs is erased and retyped. Moving back to the first row puts back what you typed. While a suggestion from the list is on the line, the list stays as it is; typing anything else makes a new list for the new line. Enter is never intercepted, so it runs whatever is on the line.

## Starting from your startup file

`compl setup` adds a line to your shell's startup file that runs `compl init`. The snippet it prints starts `compl run` for interactive terminals and exits the outer shell when the session ends. If compl cannot start, or exits with an error, you are left in the outer shell as normal. Inside the session the variable `COMPL_SESSION` is set, so the startup line does nothing there.
