
SUMMARY = "Docutils -- Python Documentation Utilities"
HOMEPAGE = "None"
AUTHOR = "None <David Goodger <goodger@python.org>>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://COPYING.rst;md5=ce467b04b35c7ac3429b6908fc8b318e"

SRC_URI = "https://files.pythonhosted.org/packages/4a/c0/89fe6215b443b919cb98a5002e107cb5026854ed1ccb6b5833e0768419d1/docutils-0.22.2.tar.gz"
SRC_URI[md5sum] = "c17270dd0ae8708360286673b1c71848"
SRC_URI[sha256sum] = "9fdb771707c8784c8f2728b67cb2c691305933d68137ef95a75db5f4dfbc213d"

S = "${WORKDIR}/docutils-0.22.2"

RDEPENDS_${PN} = ""

inherit setuptools3
