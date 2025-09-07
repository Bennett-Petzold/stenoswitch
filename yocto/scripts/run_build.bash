#!/usr/bin/env bash

# Print all commands and exit on any failure
set -xeo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")" || return
source set_build.bash
source build_env.bash

# Keep printing commands
set -x
bitbake "$build"
