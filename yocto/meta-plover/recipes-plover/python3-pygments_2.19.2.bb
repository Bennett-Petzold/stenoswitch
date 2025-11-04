
SUMMARY = "Pygments is a syntax highlighting package written in Python."
HOMEPAGE = "None"
AUTHOR = "None <Georg Brandl <georg@python.org>>"
LICENSE = "BSD-2-Clause"
LIC_FILES_CHKSUM = "file://LICENSE;md5=36a13c90514e2899f1eba7f41c3ee592"

SRC_URI = "https://files.pythonhosted.org/packages/b0/77/a5b8c569bf593b0140bde72ea885a803b82086995367bf2037de0159d924/pygments-2.19.2.tar.gz"
SRC_URI[md5sum] = "79260d1c566a507953a81d24b1c51c72"
SRC_URI[sha256sum] = "636cb2477cec7f8952536970bc533bc43743542f70392ae026374600add5b887"

S = "${WORKDIR}/pygments-2.19.2"

RDEPENDS_${PN} = ""

inherit setuptools3
