# Fish completions for rustygrep

complete -c rustygrep -f

# Flags
complete -c rustygrep -l format -s f -d 'Output format' -a 'pretty json llm'
complete -c rustygrep -l llm -d 'Token-compressed output for LLM agents'
complete -c rustygrep -l json -d 'JSON Lines output'
complete -c rustygrep -l json-file -d 'JSON per-file output'
complete -c rustygrep -l llm-budget -d 'Cap output at N tokens' -r
complete -c rustygrep -l llm-no-truncate -d 'Disable line truncation'
complete -c rustygrep -l top -d 'Show top N files by match count' -r
complete -c rustygrep -s i -l ignore-case -d 'Case insensitive search'
complete -c rustygrep -s w -l word-regexp -d 'Whole word match'
complete -c rustygrep -s c -l count -d 'Match count only'
complete -c rustygrep -s l -l files-with-matches -d 'Files with matches only'
complete -c rustygrep -s A -l after-context -d 'Context lines after match' -r
complete -c rustygrep -s B -l before-context -d 'Context lines before match' -r
complete -c rustygrep -s C -l context -d 'Context lines around match' -r
complete -c rustygrep -s t -l type -d 'Filter by file type' -r -a '(string join " " rs py js ts go java c cpp rb sh)'
complete -c rustygrep -s T -l type-not -d 'Exclude file type' -r
complete -c rustygrep -s M -l max-columns -d 'Truncate long lines at N columns' -r
complete -c rustygrep -l hidden -d 'Search hidden files'
complete -c rustygrep -l no-ignore -d 'Do not respect .gitignore'
complete -c rustygrep -l no-binary -d 'Search binary files'
complete -c rustygrep -s v -l invert-match -d 'Invert match'
complete -c rustygrep -s j -l threads -d 'Number of parallel threads' -r
complete -c rustygrep -l no-color -d 'No color output'
complete -c rustygrep -l max-matches -d 'Maximum matches per file' -r
complete -c rustygrep -l help -d 'Show help'
complete -c rustygrep -l version -d 'Show version'
