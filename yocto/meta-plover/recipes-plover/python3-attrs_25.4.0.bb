
SUMMARY = "Classes Without Boilerplate"
HOMEPAGE = "None"
AUTHOR = "None <Hynek Schlawack <hs@ox.cx>>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://LICENSE;md5=5e55731824cf9205cfabeab9a0600887"

SRC_URI = "https://files.pythonhosted.org/packages/6b/5c/685e6633917e101e5dcb62b9dd76946cbb57c26e133bae9e0cd36033c0a9/attrs-25.4.0.tar.gz"
SRC_URI[md5sum] = "6197561dfec99660529830edcfeee0ba"
SRC_URI[sha256sum] = "16d5969b87f0859ef33a48b35d55ac1be6e42ae49d5e853b597db70c35c57e11"

S = "${WORKDIR}/attrs-25.4.0"

RDEPENDS_${PN} = ""

inherit setuptools3
