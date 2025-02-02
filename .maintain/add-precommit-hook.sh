#!/bin/sh

# check that rustfmt installed, or else this hook doesn't make much sense
command -v rustfmt >/dev/null 2>&1 || { echo >&2 "Rustfmt is required but it's not installed. Aborting."; exit 1; }

# write a whole script to pre-commit hook
# NOTE: it will overwrite pre-commit file!
cat > .git/hooks/pre-commit <<'EOF'
#!/bin/bash -e
declare -a rust_files=()
files=$(git diff --name-only --staged)
echo 'Formatting source files'
for file in $files; do
    if [ ! -f "${file}" ]; then
        continue
    fi
    if [[ "${file}" == *.rs ]]; then
        rust_files+=("${file}")
    fi
done
if [ ${#rust_files[@]} -ne 0 ]; then
    command -v rustfmt >/dev/null 2>&1 || { echo >&2 "Rustfmt is required but it's not installed. Aborting."; exit 1; }
    $(command -v rustfmt) ${rust_files[@]} &
fi
wait
if [ ${#rust_files[@]} -ne 0 ]; then
    git add ${rust_files[@]}
    echo "Formatting done, changed files: ${rust_files[@]}"
else
    echo "No changes, formatting skipped"
fi
EOF

chmod +x .git/hooks/pre-commit

echo "Hooks updated"
