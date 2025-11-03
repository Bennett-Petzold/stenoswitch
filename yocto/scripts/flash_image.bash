#!/usr/bin/env bash

set -xeo pipefail

echo 'This assumes /dev/mmcblk0 is the target media!'
echo "It makes a lot of assumptions based on the author's machine."

# https://stackoverflow.com/a/34642589
if (return 2>/dev/null); then
	echo 'This script should not be sourced!'
	exit 1
fi

SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"
cd "$SCRIPT_DIR" || return
./make_image.bash "$@"

pv -o /dev/mmcblk0 "$(find ../build/wic-builds/ | tac | grep -m 1 -v '\.p.*$')"
