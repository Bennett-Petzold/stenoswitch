
SUMMARY = "A small Python module for determining appropriate platform-specific dirs, e.g. a "user data dir"."
HOMEPAGE = "http://github.com/ActiveState/appdirs"
AUTHOR = "Trent Mick <trentm@gmail.com>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE.txt;md5=31625363c45eb0c67c630a2f73e438e4"

SRC_URI = "https://files.pythonhosted.org/packages/d7/d8/05696357e0311f5b5c316d7b95f46c669dd9c15aaeecbb48c7d0aeb88c40/appdirs-1.4.4.tar.gz"
SRC_URI[md5sum] = "d6bca12613174185dd9abc8a29f4f012"
SRC_URI[sha256sum] = "7d5d0167b2b1ba821647616af46a749d1c653740dd0d2415100fe26e27afdf41"

S = "${WORKDIR}/appdirs-1.4.4"

RDEPENDS_${PN} = ""

inherit setuptools3
