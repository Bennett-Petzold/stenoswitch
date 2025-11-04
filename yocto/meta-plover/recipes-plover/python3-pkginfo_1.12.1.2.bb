
SUMMARY = "Query metadata from sdists / bdists / installed packages."
HOMEPAGE = "https://code.launchpad.net/~tseaver/pkginfo/trunk"
AUTHOR = "Tres Seaver, Agendaless Consulting <tseaver@agendaless.com>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE.txt;md5=6fc86b61c6077306ca1f5edc8edcc490"

SRC_URI = "https://files.pythonhosted.org/packages/24/03/e26bf3d6453b7fda5bd2b84029a426553bb373d6277ef6b5ac8863421f87/pkginfo-1.12.1.2.tar.gz"
SRC_URI[md5sum] = "021f56d78ec93965b21e98bc3a3ab370"
SRC_URI[sha256sum] = "5cd957824ac36f140260964eba3c6be6442a8359b8c48f4adf90210f33a04b7b"

S = "${WORKDIR}/pkginfo-1.12.1.2"

RDEPENDS_${PN} = ""

inherit setuptools3
