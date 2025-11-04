
SUMMARY = "Backported and Experimental Type Hints for Python 3.9+"
HOMEPAGE = "None"
AUTHOR = "None <"Guido van Rossum, Jukka Lehtosalo, Łukasz Langa, Michael Lee" <levkivskyi@gmail.com>>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://LICENSE;md5=fcf6b249c2641540219a727f35d8d2c2"

SRC_URI = "https://files.pythonhosted.org/packages/72/94/1a15dd82efb362ac84269196e94cf00f187f7ed21c242792a923cdb1c61f/typing_extensions-4.15.0.tar.gz"
SRC_URI[md5sum] = "079e529d0b271647f3ec2720aee8fc65"
SRC_URI[sha256sum] = "0cea48d173cc12fa28ecabc3b837ea3cf6f38c6d1136f85cbaaf598984861466"

S = "${WORKDIR}/typing_extensions-4.15.0"

RDEPENDS_${PN} = ""

inherit setuptools3
