# Troubleshooting

## Suggestions never show

- Check that you are in a session: `echo $AUTOCOMPLETE_SESSION` (or `$env:AUTOCOMPLETE_SESSION` in PowerShell) prints `1`. If not, run `autocomplete setup` and open a new terminal, or start one by hand with `autocomplete run`.
- Check that suggestions exist for the line: `autocomplete complete "cd "` lists what autocomplete would offer.
- Something set your prompt after the startup line ran. Move the line `autocomplete setup` added to the end of the startup file.

## Suggestions show in the wrong place

autocomplete asks the terminal where the cursor is when it starts. A terminal that does not answer that request (very rare) leaves autocomplete assuming the top-left corner until the screen is cleared. Run `clear` once to line things up.

## The startup line opens a second shell

That is how it works: the startup line starts `autocomplete run`, which starts your shell again inside the session. When you exit, the outer shell exits too. To avoid loading your config twice, set your terminal to run `autocomplete run --shell <shell>` directly; see [shells and terminals](shells.md#starting-autocomplete-from-the-terminal-instead-of-a-startup-file).

## Turning it off

- For one terminal: set `AUTOCOMPLETE_DISABLE=1` before the shell starts.
- For good: `autocomplete setup --remove`.

If a session ever misbehaves, exiting the shell returns you to the outer one.

## The session log

Every session writes what it saw (prompt markers, the command line it read, how many suggestions it found and which keys it handled) to `session.log` in autocomplete's cache directory: `%LOCALAPPDATA%\autocomplete` on Windows, `~/.cache/autocomplete` elsewhere. Each new session replaces the last one's log.

## Reporting a bug

Open an [issue](https://github.com/KrisPowers/autocomplete/issues/new/choose) with your OS, terminal, shell and version (`autocomplete --version`), what you typed, and the session log.
