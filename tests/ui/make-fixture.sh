#!/usr/bin/env bash
# Builds the folder tree the UI scenarios in docs/project/UI_SCENARIOS.md run
# against. Usage: tests/ui/make-fixture.sh <empty-or-new-directory>
set -euo pipefail

root="${1:?usage: make-fixture.sh <directory>}"
mkdir -p "$root"
if [[ -n "$(ls -A "$root")" ]]; then
    echo "refusing to write into a non-empty directory: $root" >&2
    exit 1
fi
cd "$root"

mkdir Documents Pictures Projects empty only-hidden locked
touch only-hidden/.secret Documents/inside.txt
chmod 000 locked

head -c 1536 /dev/zero > notes.txt
head -c 5242880 /dev/zero > video.mp4
head -c 300 /dev/zero > photo.JPG
head -c 123456 /dev/zero > report.pdf
head -c 42 /dev/zero > main.rs
head -c 2048 /dev/zero > backup.tar.gz
touch Makefile mystery .hidden file2.txt file10.txt
# A name that is not valid UTF-8 and contains a line break.
touch "$(printf 'bad\xff\nname.txt')"

touch -d "2024-03-01 09:15" notes.txt
touch -d "2025-12-24 18:00" video.mp4
touch -d "2023-07-14 12:00" photo.JPG
touch -d "2026-01-05 07:30" Documents

echo "fixture ready in $root (run 'chmod 755 $root/locked' before deleting it)"
