# Shells and terminals

autocomplete runs your shell inside its own pseudo terminal, so it works in any terminal that runs a shell: macOS Terminal, iTerm2, Windows Terminal, the Windows console, GNOME Terminal, Konsole, xterm, Alacritty, kitty, WezTerm, Ghostty, tmux, VS Code's terminal and others. It needs a terminal that understands standard VT escape sequences, which every current terminal does.

## Shells

| Shell | Startup file used by `autocomplete setup` | How the prompt markers load |
| --- | --- | --- |
| bash | `~/.bashrc` (Linux), `~/.bash_profile` (macOS) | `bash --rcfile`, which sources your usual files first |
| zsh | `$ZDOTDIR/.zshrc` or `~/.zshrc` | A temporary `ZDOTDIR` whose files source your own |
| fish | `~/.config/fish/conf.d/autocomplete.fish` | `fish -C`, after your config |
| PowerShell 7 | `$PROFILE` | `pwsh -NoExit -EncodedCommand`, after your profile |
| Windows PowerShell | `$PROFILE` | `powershell -NoExit -EncodedCommand`, after your profile |
| cmd | none | The `PROMPT` variable |

The integration scripts live in [`shell/`](../shell). They wrap your prompt rather than replace it, and re-add their marker if a prompt theme such as Starship, Oh My Posh, Powerlevel10k or Oh My Zsh rebuilds the prompt.

### bash

Login shells (the default in macOS Terminal) source `/etc/profile` and then the first of `~/.bash_profile`, `~/.bash_login` and `~/.profile`. Other shells source `/etc/bash.bashrc` and `~/.bashrc`. The marker function is appended to `PROMPT_COMMAND`.

### zsh

`zsh-autosuggestions` is turned off inside the session, since it draws ghost text in the same place. Everything else in your config, including plugin managers and themes, loads as usual.

### fish

fish's built-in autosuggestions are turned off inside the session for the same reason. Your `fish_prompt` is wrapped and still receives the real exit status.

### PowerShell

PSReadLine's predictions (`PredictionSource`) are turned off inside the session. The integration is passed as an encoded command, so it runs under any execution policy. Put the `autocomplete setup` line after anything else that sets your `prompt` function, such as Oh My Posh, so it wraps the final prompt. `setup` appends it to the end of your profile, which does this already.

### cmd

cmd has no startup file, so `autocomplete setup` does not apply. Point your terminal profile at autocomplete instead. In Windows Terminal, set a profile's command line to:

```text
autocomplete.exe run --shell cmd
```

Your own `PROMPT` is kept; the markers are added around it.

## Starting autocomplete from the terminal instead of a startup file

Any terminal that lets you choose the command for a new tab or window can start autocomplete directly, which avoids loading the shell twice:

```text
autocomplete run --shell pwsh
```

Add `--login` for a login shell.

## Nested shells

Shells started inside a session (running `bash` from zsh, for example) do not get suggestions, because the session is already in use by the outer shell. Exit back to the outer shell to get them again.
