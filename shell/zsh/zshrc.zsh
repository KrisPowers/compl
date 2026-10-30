# Loads your .zshrc, then adds the prompt markers compl reads. ZDOTDIR
# stays pointed at your directory from here on, so .zlogin and history work
# as usual.
ZDOTDIR=$COMPL_ZDOTDIR
[[ -r $ZDOTDIR/.zshrc ]] && . $ZDOTDIR/.zshrc

__compl_precmd() {
    printf '\033]6973;A;%s\007\033]6973;P;%s\007' "$PWD" "$PATH"
    # Prompt themes rebuild PROMPT, so the marker is re-added when missing.
    [[ $PROMPT == *'6973;B'* ]] || PROMPT="${PROMPT}%{"$'\033]6973;B\007'"%}"
}

# zsh-autosuggestions draws its own ghost text where ours goes.
typeset -g _ZSH_AUTOSUGGEST_DISABLED=1

autoload -Uz add-zsh-hook
add-zsh-hook precmd __compl_precmd
