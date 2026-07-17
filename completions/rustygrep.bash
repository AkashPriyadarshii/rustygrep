#!/usr/bin/env bash

_rustygrep_completions() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    opts="--format --llm --json --json-file --llm-budget --llm-no-truncate --top --ignore-case --word-regexp --count --files-with-matches --after-context --before-context --context --type --type-not --max-columns --hidden --no-ignore --no-binary --invert-match --threads --no-color --max-matches --help --version"

    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi

    # Complete file/directory paths
    COMPREPLY=( $(compgen -f -- "${cur}") )
    return 0
}

complete -F _rustygrep_completions rustygrep
