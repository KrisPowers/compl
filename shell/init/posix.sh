# Starts autocomplete for interactive terminals. If it cannot start, or exits
# with an error, you stay in this shell.
if [ -z "$AUTOCOMPLETE_SESSION" ] && [ -z "$AUTOCOMPLETE_DISABLE" ] && [ -t 0 ] && [ -t 1 ] && [ "$TERM" != dumb ]; then
    case $- in
        *i*)
            __autocomplete_login=
            __LOGIN__ && __autocomplete_login=--login
            __EXE__ run --shell __SHELL__ $__autocomplete_login && exit
            unset __autocomplete_login
            ;;
    esac
fi
