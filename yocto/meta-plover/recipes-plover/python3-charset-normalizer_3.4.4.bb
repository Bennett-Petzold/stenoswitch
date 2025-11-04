
SUMMARY = "The Real First Universal Charset Detector. Open, modern and actively maintained alternative to Chardet."
HOMEPAGE = "None"
AUTHOR = "None <"Ahmed R. TAHRI" <tahri.ahmed@proton.me>>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=48178f3fc1374ad7e830412f812bde05"

SRC_URI = "https://files.pythonhosted.org/packages/13/69/33ddede1939fdd074bce5434295f38fae7136463422fe4fd3e0e89b98062/charset_normalizer-3.4.4.tar.gz"
SRC_URI[md5sum] = "3c73a3c5c05a896c1169d8b5298dc4e5"
SRC_URI[sha256sum] = "94537985111c35f28720e43603b8e7b43a6ecfb2ce1d3058bbe955b73404e21a"

S = "${WORKDIR}/charset_normalizer-3.4.4"

RDEPENDS_${PN} = ""

inherit setuptools3
