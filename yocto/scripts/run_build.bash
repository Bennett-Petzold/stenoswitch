#!/usr/bin/env bash

set -xeo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")" || return
source set_build.bash
source build_env.bash

# Print all commands and exit on any failure
set -xeo pipefail

bitbake "${build_args[@]}" "$build"
