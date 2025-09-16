# Fix for flex malloc in cross compile
# https://github.com/westes/flex/pull/674#issuecomment-2906992608
export CFLAGS_FOR_BUILD="${BUILD_CFLAGS} -std=gnu17"
