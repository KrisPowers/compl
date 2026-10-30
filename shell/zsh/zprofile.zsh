# Loads your .zprofile for login shells. See .zshenv.
ZDOTDIR=$COMPL_ZDOTDIR
[[ -r $ZDOTDIR/.zprofile ]] && . $ZDOTDIR/.zprofile
ZDOTDIR=$COMPL_INTEGRATION
