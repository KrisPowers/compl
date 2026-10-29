# autocomplete sources this after your fish configuration. It adds the prompt
# markers autocomplete reads.

# fish's own autosuggestions draw where ours go.
set -g fish_autosuggestion_enabled 0

function __autocomplete_mark --on-event fish_prompt
    printf '\e]6973;A;%s\a\e]6973;P;%s\a' $PWD (string join : -- $PATH)
end

function __autocomplete_status
    return $argv[1]
end

if functions -q fish_prompt; and not functions -q __autocomplete_user_prompt
    functions -c fish_prompt __autocomplete_user_prompt
end

function fish_prompt
    set -l last $status
    if functions -q __autocomplete_user_prompt
        # Hand the user's prompt the real exit status of the last command.
        __autocomplete_status $last
        __autocomplete_user_prompt
    else
        printf '%s> ' (prompt_pwd)
    end
    printf '\e]6973;B\a'
end
