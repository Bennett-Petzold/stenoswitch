LICENSE = "GPL-2"
LIC_FILES_CHKSUM = "file://LICENSE.txt;md5=b234ee4d69f5fce4486a80fdaf4a4263"
HOMEPAGE = "https://www.openstenoproject.org/plover/"

SRC_URI[sha512sum] = "49c50eaa5b09c27315f6c07bd9d90bcd637d41850701dfdb7cf6f4b0df6cfcd00d7332c3150c2abb90b0d410775f5822f9be8219a5e075db717a4e08155691a2"

inherit pypi setuptools3

SRC_URI:append = "https://raw.githubusercontent.com/openstenoproject/plover/refs/heads/main/LICENSE.txt;name=license"
SRC_URI[license.sha512sum] = "7a1dba2c878f7a2395175da465a20103ac7e33b145c662373114cd84a29b2b1f0a45e04e6140cdcdf56c11e2e0260dab90a2cc29babc0e6cbf1676b1dd377af2"

RDEPENDS:${PN} += " python3-pyside6"
