
SUMMARY = "Simple RTF tokenizer"
HOMEPAGE = "https://github.com/benoit-pierre/rtf_tokenize"
AUTHOR = "Benoit Pierre <benoit.pierre@gmail.com>"
LICENSE = "GPL-2.0-or-later"
LIC_FILES_CHKSUM = "file://LICENSE.txt;md5=b234ee4d69f5fce4486a80fdaf4a4263"

SRC_URI = "https://files.pythonhosted.org/packages/a2/e1/c700f2043567a9fce17adb2ae7d91f0f7f88e8e555eff5b8436b8f9cf6aa/rtf_tokenize-1.0.1.tar.gz"
SRC_URI[md5sum] = "cc3b6ba936793405d39db62ff1a0db22"
SRC_URI[sha256sum] = "9020aa801502b5de60be2b7709b9ce4cb29cd70df6f5fc4953315cf158035ad2"

S = "${WORKDIR}/rtf_tokenize-1.0.1"

RDEPENDS_${PN} = ""

inherit setuptools3
