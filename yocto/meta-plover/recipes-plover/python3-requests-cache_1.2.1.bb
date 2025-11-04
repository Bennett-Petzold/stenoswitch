
SUMMARY = "A persistent cache for python requests"
HOMEPAGE = "https://github.com/requests-cache/requests-cache"
AUTHOR = "Roman Haritonov <None>"
LICENSE = "BSD-2-Clause"
LIC_FILES_CHKSUM = "file://LICENSE;md5=66ca615c6f22205d5254d6c230305c92"

SRC_URI = "https://files.pythonhosted.org/packages/1a/be/7b2a95a9e7a7c3e774e43d067c51244e61dea8b120ae2deff7089a93fb2b/requests_cache-1.2.1.tar.gz"
SRC_URI[md5sum] = "27038cb33985f5b144cf32107151921a"
SRC_URI[sha256sum] = "68abc986fdc5b8d0911318fbb5f7c80eebcd4d01bfacc6685ecf8876052511d1"

S = "${WORKDIR}/requests_cache-1.2.1"

RDEPENDS_${PN} = "python3-attrs python3-cattrs python3-platformdirs python3-requests python3-url-normalize python3-urllib3"

inherit setuptools3
