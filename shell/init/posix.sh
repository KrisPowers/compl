# Starts compl for interactive terminals. If it cannot start, or exits
# with an error, you stay in this shell.
if [ -z "$COMPL_SESSION" ] && [ -z "$COMPL_DISABLE" ] && [ -t 0 ] && [ -t 1 ] && [ "$TERM" != dumb ]; then
    case $- in
        *i*)
            __compl_login=
            __LOGIN__ && __compl_login=--login
            __EXE__ run --shell __SHELL__ $__compl_login && exit
            unset __compl_login
            ;;
    esac
fi
