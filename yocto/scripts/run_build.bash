#!/usr/bin/env bash

PRIMARY_BUILD='core-image-minimal'
DEBUG_BUILD='core-image-minimal-dev'
# TODO PGO builds
# Collect is run on a system to collect real usage data.
PGO_COLLECT_BUILD='core-image-minimal'
# Apply uses the data from collect to make a more optimized image.
PGO_APPLY_BUILD='core-image-minimal'

# https://stackoverflow.com/a/34642589
if (return 2>/dev/null); then
	echo 'This script should not be sourced!'
	exit 1
fi

echo "Valid targets: debug,simulate,pgo_collect,pgo_apply."
echo "If no target is given, the default build is run."
echo "Set MACHINE=qemuarm64 to build a simulator image."

case "${1,,}" in
	'') build="${PRIMARY_BUILD}";;
	debug) build="${DEBUG_BUILD}";;
	pgo_collect|pgo-collect) build="${PGO_COLLECT_BUILD}";;
	pgo_apply|pgo-apply) build="${PGO_APPLY_BUILD}";;
	*) echo "${1} is not a valid target!"
	exit 1;;
esac

build="${PRIMARY_BUILD}"

# Print all commands and exit on any failure
set -xeo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")" || return
source build_env.bash

# Keep printing commands
set -x
bitbake "$build"
