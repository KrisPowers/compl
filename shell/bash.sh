# autocomplete passes this file to bash with --rcfile. It loads your usual
# startup files first, then adds the prompt markers autocomplete reads:
#   ESC ] 6973 ; A ; <cwd> BEL   a prompt is about to be drawn
#   ESC ] 6973 ; P ; <PATH> BEL  the PATH commands are found on
#   ESC ] 6973 ; B BEL           the prompt is drawn, input starts here

if [ -n "$AUTOCOMPLETE_LOGIN" ]; then
    unset AUTOCOMPLETE_LOGIN
    [ -r /etc/profile ] && . /etc/profile
    for __autocomplete_file in ~/.bash_profile ~/.bash_login ~/.profile; do
        if [ -r "$__autocomplete_file" ]; then
            . "$__autocomplete_file"
            break
        fi
    done
    unset __autocomplete_file
else
    [ -r /etc/bash.bashrc ] && . /etc/bash.bashrc
    [ -r ~/.bashrc ] && . ~/.bashrc
fi

__autocomplete_precmd() {
    local last=$?
    printf '\033]6973;A;%s\007\033]6973;P;%s\007' "$PWD" "$PATH"
    # Prompt themes rebuild PS1 on every prompt, so the marker is re-added
    # whenever it goes missing.
    case "$PS1" in
        *'6973;B'*) ;;
        *) PS1="$PS1"'\[\033]6973;B\007\]' ;;
    esac
    return $last
}

PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND
}__autocomplete_precmd"
