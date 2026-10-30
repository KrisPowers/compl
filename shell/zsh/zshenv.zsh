# compl points ZDOTDIR at a directory holding this file, .zprofile and
# .zshrc. Each one loads your own file of the same name from your real
# ZDOTDIR, so your configuration runs unchanged.
ZDOTDIR=${COMPL_ZDOTDIR:-$HOME}
[[ -r $ZDOTDIR/.zshenv ]] && . $ZDOTDIR/.zshenv
# Your .zshenv may have moved ZDOTDIR; the later files follow it.
COMPL_ZDOTDIR=$ZDOTDIR
ZDOTDIR=$COMPL_INTEGRATION
