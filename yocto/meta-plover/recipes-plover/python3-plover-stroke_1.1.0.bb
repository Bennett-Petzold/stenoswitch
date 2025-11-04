
SUMMARY = "Stroke handling helper library for Plover"
HOMEPAGE = "https://github.com/benoit-pierre/plover_stroke"
AUTHOR = "Benoit Pierre <benoit.pierre@gmail.com>"
LICENSE = "GPL-2.0-or-later"
LIC_FILES_CHKSUM = "file://setup.py;md5=f9fb06b83a6f8ebbe37b4d70e2226c3b"

SRC_URI = "https://files.pythonhosted.org/packages/cc/53/92635d8bf00b883bfbc6ab9dd48b6df2ed01c241379fe99f063a41530cab/plover_stroke-1.1.0.tar.gz"
SRC_URI[md5sum] = "7cea9fc27cbe92f85ab372aee67fab86"
SRC_URI[sha256sum] = "de03b23f4aee66b65f69f7d4ecc4233681b43541502d86bf14fde29eaa72d153"

S = "${WORKDIR}/plover_stroke-1.1.0"

RDEPENDS_${PN} = ""

inherit setuptools3
