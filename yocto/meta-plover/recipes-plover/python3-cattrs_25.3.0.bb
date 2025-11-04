
SUMMARY = "Composable complex class support for attrs and dataclasses."
HOMEPAGE = "None"
AUTHOR = "None <Tin Tvrtkovic <tinchester@gmail.com>>"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=12efd5ce6c6c43c4ead370bd15f93560"

SRC_URI = "https://files.pythonhosted.org/packages/6e/00/2432bb2d445b39b5407f0a90e01b9a271475eea7caf913d7a86bcb956385/cattrs-25.3.0.tar.gz"
SRC_URI[md5sum] = "9b7b27f64ada35523229b778f4199043"
SRC_URI[sha256sum] = "1ac88d9e5eda10436c4517e390a4142d88638fe682c436c93db7ce4a277b884a"

S = "${WORKDIR}/cattrs-25.3.0"

RDEPENDS_${PN} = "python3-attrs python3-typing-extensions"

inherit setuptools3
