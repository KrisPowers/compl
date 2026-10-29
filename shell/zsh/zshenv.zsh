# autocomplete points ZDOTDIR at a directory holding this file, .zprofile and
# .zshrc. Each one loads your own file of the same name from your real
# ZDOTDIR, so your configuration runs unchanged.
ZDOTDIR=${AUTOCOMPLETE_ZDOTDIR:-$HOME}
[[ -r $ZDOTDIR/.zshenv ]] && . $ZDOTDIR/.zshenv
# Your .zshenv may have moved ZDOTDIR; the later files follow it.
AUTOCOMPLETE_ZDOTDIR=$ZDOTDIR
ZDOTDIR=$AUTOCOMPLETE_INTEGRATION
