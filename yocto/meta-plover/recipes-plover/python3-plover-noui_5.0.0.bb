
SUMMARY = "Open Source Stenography Software"
HOMEPAGE = "http://www.openstenoproject.org/"
AUTHOR = "Joshua Harlan Lifton <joshua.harlan.lifton@gmail.com>"
LICENSE = "GPL-2.0-or-later"
LIC_FILES_CHKSUM = "file://LICENSE.txt;md5=b234ee4d69f5fce4486a80fdaf4a4263"

SRC_URI = "https://files.pythonhosted.org/packages/eb/56/ad9bca464f406ef042302e6992999e86894dc6299b5d8a7ccbf553777ee5/plover-5.0.0.tar.gz"
SRC_URI[md5sum] = "34b77c9fc76f547af14e5ea6605da6c5"
SRC_URI[sha256sum] = "5edb3641dacf8593837ae28d4a9e1be6edb4ca09f4e51920c4c3f46a34d3df87"

S = "${WORKDIR}/plover-5.0.0"

DEPENDS = " sed-native"

RDEPENDS:${PN} = "python3-modules python3-appdirs python3-pkginfo python3-plover-stroke python3-pygments python3-pyserial python3-rtf-tokenize python3-wcwidth python3-evdev python3-psutil"

SETUPTOOLS_BUILD_ARGS += " build_py"

do_configure:append () {
    sed -i '/build_ui/d' setup.py
    sed -i '/compile_catalog/d' setup.py
    sed -i '/gui_qt/d' \
        setup.py setup.cfg MANIFEST.in plover.egg-info/SOURCES.txt
    rm -rf plover/gui_qt/ build/lib/plover/gui_qt
}

inherit setuptools3
