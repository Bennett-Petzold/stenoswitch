
SUMMARY = "URL normalization for Python"
HOMEPAGE = "None"
AUTHOR = "None <Nikolay Panov <github@npanov.com>>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=0355248f9f4025eb234b21ac43b9ad7a"

SRC_URI = "https://files.pythonhosted.org/packages/80/31/febb777441e5fcdaacb4522316bf2a527c44551430a4873b052d545e3279/url_normalize-2.2.1.tar.gz"
SRC_URI[md5sum] = "2894fd86ec1ea95ef5be3cfaf4adf9df"
SRC_URI[sha256sum] = "74a540a3b6eba1d95bdc610c24f2c0141639f3ba903501e61a52a8730247ff37"

S = "${WORKDIR}/url_normalize-2.2.1"

RDEPENDS:${PN} = "python3-idna"

inherit setuptools3
