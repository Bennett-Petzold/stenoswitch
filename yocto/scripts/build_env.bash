#!/usr/bin/env bash

GIT_SUBMODULES=('poky' 'meta-raspberrypi')

# https://stackoverflow.com/a/34642589
if ! (return 2>/dev/null); then
	echo 'This script must be sourced!'
	exit 1
fi

# Print all commands for visibility
trap 'set +x' EXIT
set -x

cd "$(dirname "${BASH_SOURCE[0]}")"/.. || return
git submodule update --init --recursive "${GIT_SUBMODULES[@]}" || return
cd poky || return
source oe-init-build-env ../build/ || return

# Can now be used as an interactive shell
set +x
