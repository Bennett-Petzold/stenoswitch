#!/usr/bin/env bash

set -xeo pipefail

# https://stackoverflow.com/a/34642589
if (return 2>/dev/null); then
	echo 'This script should not be sourced!'
	exit 1
fi

SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"
cd "$SCRIPT_DIR" || return
./run_build.bash "$@"
source set_build.bash
source build_env.bash

# Print all commands and exit on any failure
set -xeo pipefail

# Build the distribution for packaging
bitbake wic-tools

# Package the distribution to an image
mkdir -p wic-builds/
cd wic-builds
wic create "${SCRIPT_DIR}/../stenoswitch.wks" -e "$build"
