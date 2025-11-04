
SUMMARY = "HTTP library with thread-safe connection pooling, file post, and more."
HOMEPAGE = "None"
AUTHOR = "None <Andrey Petrov <andrey.petrov@shazow.net>>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://LICENSE.txt;md5=52d273a3054ced561275d4d15260ecda"

SRC_URI = "https://files.pythonhosted.org/packages/15/22/9ee70a2574a4f4599c47dd506532914ce044817c7752a79b6a51286319bc/urllib3-2.5.0.tar.gz"
SRC_URI[md5sum] = "2b8a86438e4d35fbc90572dbdb424759"
SRC_URI[sha256sum] = "3fc47733c7e419d4bc3f6b3dc2b4f890bb743906a30d56ba4a5bfa4bbff92760"

S = "${WORKDIR}/urllib3-2.5.0"

RDEPENDS_${PN} = ""

inherit setuptools3
