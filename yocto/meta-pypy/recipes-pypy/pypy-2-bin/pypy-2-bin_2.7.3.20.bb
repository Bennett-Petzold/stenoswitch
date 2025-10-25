LICENSE = "GPL-3.0-or-later"
LIC_FILES_CHKSUM = "file://LICENSE;md5=3aece17978106d8a8c8b1f5e3c4ecf34"

FILES:${PN} = "${libdir}/* ${bindir}/* ${includedir}/* /usr/local/bin/python /usr/lib-python /usr/lib_pypy"

OVERRIDES = "architecture:x86_64"

SRC_URI = "https://downloads.python.org/pypy/pypy2.7-v7.3.20-aarch64.tar.bz2;name=aarch64"
SRC_URI[aarch64.sha256sum] = "f22a1be607deeaa4f9be6bc63aae09fe4fb5b990d6a23aa4e7c5960dc5d93c96"
SRC_URI:x86_64 = "https://downloads.python.org/pypy/pypy2.7-v7.3.20-linux64.tar.bz2;name=x86"
SRC_URI[x86.sha256sum] = "aa3bb92dbb529fa2d4920895b16d67a810b0c709207857d56cfe4a6e3b41e02a"

DEPENDS = "bash libxft expat"
RPROVIDES:${PN} += "python python3 python3-core python3-misc pypy pypy2"

COMPATIBLE_HOST = "^(armv|aarch64|x86_64).*$"

BBCLASSEXTEND += "native"

SOLIBS = ".so"
FILES_SOLIBSDEV = ""

SYSROOT_DIRS:append = " ${bindir} ${bindir}/../lib-python ${bindir}/../lib_pypy"

INSANE_SKIP:${PN} += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-dbg += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-dev += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-staticdev += "already-stripped file-rdeps libdir dev-elf"

python do_move_lic () {
    bb.build.exec_func("move_lic", d)
}

move_lic () {
    cp ${WORKDIR}/pypy2*-*/LICENSE ${S}/
}

addtask do_move_lic after do_unpack before do_populate_lic

do_install () {
    mkdir -p ${D}${libdir} ${D}${bindir} ${D}${includedir} ${D}/usr/local/bin/ ${D}${bindir}/../lib-python ${D}${bindir}/../lib_pypy

    cp -r --no-preserve=ownership ${WORKDIR}/pypy2*-*/lib/. ${D}${libdir}
    rm ${D}${libdir}/libexpat.so.1 ${D}${libdir}/libsqlite3.so.0

    cp -r --no-preserve=ownership ${WORKDIR}/pypy2*-*/bin/. ${D}${bindir}
    cp -r --no-preserve=ownership ${WORKDIR}/pypy2*-*/include/. ${D}${includedir}
    cp -r --no-preserve=ownership ${WORKDIR}/pypy2*-*/lib-python/. ${D}${bindir}/../lib-python
    cp -r --no-preserve=ownership ${WORKDIR}/pypy2*-*/lib_pypy/. ${D}${bindir}/../lib_pypy

    ln -sr ${D}${bindir}/python ${D}/usr/local/bin/python 
}
