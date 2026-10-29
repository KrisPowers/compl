# autocomplete runs this after your PowerShell profile. It adds the prompt
# markers autocomplete reads.

# PSReadLine's own predictions draw where ours go.
if (Get-Command Set-PSReadLineOption -ErrorAction SilentlyContinue) {
    try { Set-PSReadLineOption -PredictionSource None } catch { }
}

$global:__AutocompleteUserPrompt = $function:prompt

function global:prompt {
    $text = (& $global:__AutocompleteUserPrompt) -join ''
    $e = [char]27
    $b = [char]7
    $cwd = $executionContext.SessionState.Path.CurrentLocation.ProviderPath
    "$e]6973;A;$cwd$b$e]6973;P;$env:PATH$b$text$e]6973;B$b"
}
