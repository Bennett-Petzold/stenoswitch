
SUMMARY = "Python binding to Ammonia HTML sanitizer Rust crate"
HOMEPAGE = "None"
AUTHOR = "None <messense <messense@icloud.com>>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=1095f9c2128d0f3bb8d88f92e25dd639"

SRC_URI = "https://files.pythonhosted.org/packages/ca/a5/34c26015d3a434409f4d2a1cd8821a06c05238703f49283ffeb937bef093/nh3-0.3.2.tar.gz"
SRC_URI[md5sum] = "f1dbd8fe8b87fb4e56318cdb17006d8e"
SRC_URI[sha256sum] = "f394759a06df8b685a4ebfb1874fb67a9cbfd58c64fc5ed587a663c0e63ec376"

S = "${WORKDIR}/nh3-0.3.2"

RDEPENDS_${PN} = ""

inherit setuptools3
