#!/usr/bin/env bash

set +x

PRIMARY_BUILD='core-image-base'
DEBUG_BUILD='core-image-full-cmdline'

# TODO PGO and Bolt builds
# Collect is run on a system to collect real usage data.
PGO_COLLECT='conf/pgo-collect.conf'
# Apply uses the data from collect to make a more optimized image.
PGO_APPLY='conf/pgo-apply.conf'
# More collection with a PGO binary.
BOLT_COLLECT='conf/bolt-collect.conf'
# Full instrumented optimization.
BOLT_APPLY='conf/bolt-apply.conf'

echo "Valid targets: release,debug."
echo "Valid extra args: pgo_collect,pgo_apply,bolt_collect,bolt_apply."
echo "See yocto/optimization.md for extra arg details."
echo "If no target is given, the default build is run."
echo "Set MACHINE=qemuarm64 to build a simulator image."

for arg in "$@"; do
	if [[ "${arg,,}" == '-h' || "${arg,,}" == '--help' ]]; then
		exit 0;
	fi
done

case "${1,,}" in
	''|regular|main|release) build="${PRIMARY_BUILD}";;
	debug) build="${DEBUG_BUILD}";;
	*) echo "${1} is not a valid target!"
	exit 1;;
esac

case "${2,,}" in
	'') build_args=();;
	pgo_collect|pgo-collect) build_args=('-R' "$PGO_COLLECT");;
	pgo_apply|pgo-apply) build_args=('-R' "$PGO_APPLY");;
	bolt_collect|bolt-collect) build_args=('-R' "$BOLT_COLLECT");;
	bolt_apply|bolt-apply) build_args=('-R' "$BOLT_APPLY");;
	*) echo "${2} is not a valid additional argument!"
	exit 2;;
esac

export build
export build_args
