#!/usr/bin/env bash

# https://stackoverflow.com/a/34642589
if (return 2>/dev/null); then
	echo 'This script should not be sourced!'
	exit 1
fi

# Print all commands and exit on any failure
set -xeo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")" || return
source set_build.bash

# Keep printing commands
set -x
bitbake wic-tools

mkdir -p wic-builds/
cd wic-builds
wic create ../stenoswitch.wks -e "$build"
