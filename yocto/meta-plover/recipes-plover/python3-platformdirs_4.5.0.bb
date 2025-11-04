
SUMMARY = "A small Python package for determining appropriate platform-specific dirs, e.g. a `user data dir`."
HOMEPAGE = "None"
AUTHOR = "None <None>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://LICENSE;md5=ea4f5a41454746a9ed111e3d8723d17a"

SRC_URI = "https://files.pythonhosted.org/packages/61/33/9611380c2bdb1225fdef633e2a9610622310fed35ab11dac9620972ee088/platformdirs-4.5.0.tar.gz"
SRC_URI[md5sum] = "e3a2646918667a859323d03fb6515975"
SRC_URI[sha256sum] = "70ddccdd7c99fc5942e9fc25636a8b34d04c24b335100223152c2803e4063312"

S = "${WORKDIR}/platformdirs-4.5.0"

RDEPENDS_${PN} = ""

inherit setuptools3
