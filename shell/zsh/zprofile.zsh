# Loads your .zprofile for login shells. See .zshenv.
ZDOTDIR=$AUTOCOMPLETE_ZDOTDIR
[[ -r $ZDOTDIR/.zprofile ]] && . $ZDOTDIR/.zprofile
ZDOTDIR=$AUTOCOMPLETE_INTEGRATION
