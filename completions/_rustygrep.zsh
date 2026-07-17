#compdef rustygrep

_rustygrep() {
    _arguments \
        '(--format -f)'{-f,--format}'[Output format]:format:(pretty json llm)' \
        '--llm[Token-compressed output for LLM agents]' \
        '--json[JSON Lines output]' \
        '--json-file[JSON per-file output]' \
        '--llm-budget[Cap output at N tokens]:number:' \
        '--llm-no-truncate[Disable line truncation]' \
        '--top[Show top N files by match count]:number:' \
        '(-i --ignore-case)'{-i,--ignore-case}'[Case insensitive search]' \
        '(-w --word-regexp)'{-w,--word-regexp}'[Whole word match]' \
        '(-c --count)'{-c,--count}'[Match count only]' \
        '(-l --files-with-matches)'{-l,--files-with-matches}'[Files with matches only]' \
        '(-A --after-context)'{-A,--after-context}'[Context lines after match]:number:' \
        '(-B --before-context)'{-B,--before-context}'[Context lines before match]:number:' \
        '(-C --context)'{-C,--context}'[Context lines around match]:number:' \
        '(-t --type)'{-t,--type}'[Filter by file type]:file type:' \
        '(-T --type-not)'{-T,--type-not}'[Exclude file type]:file type:' \
        '(-M --max-columns)'{-M,--max-columns}'[Truncate long lines at N columns]:number:' \
        '--hidden[Search hidden files]' \
        '--no-ignore[Do not respect .gitignore]' \
        '--no-binary[Search binary files]' \
        '(-v --invert-match)'{-v,--invert-match}'[Invert match]' \
        '(-j --threads)'{-j,--threads}'[Number of parallel threads]:number:' \
        '--no-color[No color output]' \
        '--max-matches[Maximum matches per file]:number:' \
        '--help[Show help]' \
        '--version[Show version]' \
        '*:pattern:_files' \
        '*:: :_files'
}

_rustygrep "$@"
