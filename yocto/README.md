# Build Scripts
* `source yocto/build_env.bash` to enter the build environment interactively.
* `scripts/run_build.bash` to build the distribution and partition.
* `scripts/make_image.bash` to create the disk image.

# Installing the Disk Image
To flash the disk image to media, write _ONLY_ the length of the disk image.
Any remaining space on the device is user scratch and retained between updates.

# Build/Release Process
Builds use the upstream hash server to save time.
Yocto images are not currently built in Actions due to the memory requirement.
Contributors are responsible for locally validating Yocto builds.
A local Yocto build is manually added to releases.
