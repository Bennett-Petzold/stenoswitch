
SUMMARY = "Internationalized Domain Names in Applications (IDNA)"
HOMEPAGE = "None"
AUTHOR = "None <Kim Davies <kim+pypi@gumleaf.org>>"
LICENSE = "Apache-2.0 "
LIC_FILES_CHKSUM = "file://LICENSE.md;md5=18a4795c19833413a7e2f1cb3cd3b143"

SRC_URI = "https://files.pythonhosted.org/packages/6f/6d/0703ccc57f3a7233505399edb88de3cbd678da106337b9fcde432b65ed60/idna-3.11.tar.gz"
SRC_URI[md5sum] = "9a9c33db960e0d35cc5b257c37dabeff"
SRC_URI[sha256sum] = "795dafcc9c04ed0c1fb032c2aa73654d8e8c5023a7df64a53f39190ada629902"

S = "${WORKDIR}/idna-3.11"

RDEPENDS_${PN} = ""

inherit setuptools3
