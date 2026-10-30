# Starts compl for interactive terminals. If it cannot start, or exits
# with an error, you stay in this shell.
if status is-interactive; and not set -q COMPL_SESSION; and not set -q COMPL_DISABLE; and test -t 0; and test -t 1
    set -l login
    status is-login; and set login --login
    __EXE__ run --shell __SHELL__ --exe (status fish-path) $login; and exit
end
