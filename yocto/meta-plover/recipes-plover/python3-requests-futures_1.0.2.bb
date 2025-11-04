
SUMMARY = "Asynchronous Python HTTP for Humans."
HOMEPAGE = "https://github.com/ross/requests-futures"
AUTHOR = "Ross McFarland <rwmcfa1@gmail.com>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://LICENSE;md5=e1e50798d0afe0e1f87594c6619a2fa5"

SRC_URI = "https://files.pythonhosted.org/packages/88/f8/175b823241536ba09da033850d66194c372c65c38804847ac9cef0239542/requests_futures-1.0.2.tar.gz"
SRC_URI[md5sum] = "cfa914d02e9f5aa7b12d6bdc4b673de2"
SRC_URI[sha256sum] = "6b7eb57940336e800faebc3dab506360edec9478f7b22dc570858ad3aa7458da"

S = "${WORKDIR}/requests_futures-1.0.2"

RDEPENDS_${PN} = "python3-requests"

inherit setuptools3
