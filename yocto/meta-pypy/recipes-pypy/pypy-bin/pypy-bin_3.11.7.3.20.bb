LICENSE = "GPL-3.0-or-later"
LIC_FILES_CHKSUM = "file://LICENSE;md5=3aece17978106d8a8c8b1f5e3c4ecf34"

FILES:${PN} = "${libdir}/* ${bindir}/* ${includedir}/* /usr/local/bin/python"

OVERRIDES = "architecture:x86_64"

SRC_URI = "https://downloads.python.org/pypy/pypy3.11-v7.3.20-aarch64.tar.bz2;name=aarch64"
SRC_URI[aarch64.sha256sum] = "9347fe691a07fd9df17a1b186554fb9d9e6210178ffef19520a579ce1f9eb741"
SRC_URI:x86_64 = "https://downloads.python.org/pypy/pypy3.11-v7.3.20-linux64.tar.bz2;name=x86"
SRC_URI[x86.sha256sum] = "1410db3a7ae47603e2b7cbfd7ff6390b891b2e041c9eb4f1599f333677bccb3e"

DEPENDS = "bash libxft expat"
RPROVIDES:${PN} += "python python3 python3-core python3-misc pypy pypy3"

COMPATIBLE_HOST = "^(armv|aarch64|x86_64).*$"

BBCLASSEXTEND += "native"

SOLIBS = ".so"
FILES_SOLIBSDEV = ""

SYSROOT_DIRS:append = " ${bindir}"

INSANE_SKIP:${PN} += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-dbg += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-dev += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-staticdev += "already-stripped file-rdeps libdir dev-elf"

python do_move_lic () {
    bb.build.exec_func("move_lic", d)
}

move_lic () {
    cp ${WORKDIR}/pypy3*-*/LICENSE ${S}/
}

addtask do_move_lic after do_unpack before do_populate_lic

do_install () {
    mkdir -p ${D}${libdir} ${D}${bindir} ${D}${includedir} ${D}/usr/local/bin/

    cp -r --no-preserve=ownership ${WORKDIR}/pypy3*-*/lib/. ${D}${libdir}
    rm ${D}${libdir}/libexpat.so.1 ${D}${libdir}/libsqlite3.so.0

    cp -r --no-preserve=ownership ${WORKDIR}/pypy3*-*/bin/. ${D}${bindir}
    cp -r --no-preserve=ownership ${WORKDIR}/pypy3*-*/include/. ${D}${includedir}

    ln -sr ${D}${bindir}/python ${D}/usr/local/bin/python 
}
