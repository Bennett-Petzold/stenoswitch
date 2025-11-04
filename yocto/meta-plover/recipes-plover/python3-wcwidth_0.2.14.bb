
SUMMARY = "Measures the displayed width of unicode strings in a terminal"
HOMEPAGE = "https://github.com/jquast/wcwidth"
AUTHOR = "Jeff Quast <contact@jeffquast.com>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=b15979c39a2543892fca8cd86b4b52cb"

SRC_URI = "https://files.pythonhosted.org/packages/24/30/6b0809f4510673dc723187aeaf24c7f5459922d01e2f794277a3dfb90345/wcwidth-0.2.14.tar.gz"
SRC_URI[md5sum] = "c179ab1aff6e3b48ac9617cf19f580d4"
SRC_URI[sha256sum] = "4d478375d31bc5395a3c55c40ccdf3354688364cd61c4f6adacaa9215d0b3605"

S = "${WORKDIR}/wcwidth-0.2.14"

RDEPENDS_${PN} = ""

inherit setuptools3
