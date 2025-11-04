
SUMMARY = "readme_renderer is a library for rendering readme descriptions for Warehouse"
HOMEPAGE = "None"
AUTHOR = "None <The Python Packaging Authority <admin@mail.pypi.org>>"
LICENSE = "Apache-2.0"
LIC_FILES_CHKSUM = "file://LICENSE;md5=8cc789b082b3d97e1ccc5261f8594d3f"

SRC_URI = "https://files.pythonhosted.org/packages/5a/a9/104ec9234c8448c4379768221ea6df01260cd6c2ce13182d4eac531c8342/readme_renderer-44.0.tar.gz"
SRC_URI[md5sum] = "bcbb9d99a7b02379041044552f180c70"
SRC_URI[sha256sum] = "8712034eabbfa6805cacf1402b4eeb2a73028f72d1166d6f5cb7f9c047c5d1e1"

S = "${WORKDIR}/readme_renderer-44.0"

RDEPENDS_${PN} = "python3-nh3 python3-docutils python3-pygments"

inherit setuptools3
