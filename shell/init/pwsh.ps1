# Starts autocomplete for interactive consoles. If it cannot start, or exits
# with an error, you stay in this shell.
if (-not $env:AUTOCOMPLETE_SESSION -and -not $env:AUTOCOMPLETE_DISABLE -and $Host.Name -eq 'ConsoleHost' -and -not [Console]::IsInputRedirected) {
    & __EXE__ run --shell __SHELL__ --exe (Get-Process -Id $PID).Path
    if ($LASTEXITCODE -eq 0) { exit }
}
