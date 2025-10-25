# CURRENTLY BROKEN! Cross compilation is complicated.
LICENSE = "GPL-3.0-or-later"
LIC_FILES_CHKSUM = "file://LICENSE;md5=3aece17978106d8a8c8b1f5e3c4ecf34"

FILES:${PN} = "${libdir}/* ${bindir}/* ${includedir}/* /usr/local/bin/python"

SRC_URI = "https://downloads.python.org/pypy/pypy3.11-v7.3.20-src.tar.bz2"
SRC_URI[sha256sum] = "7786dda760003e2ea7409c1037e50200c578ec427ce0245ac4cd758710b206fb"

DEPENDS = "bash libxft expat pypy-2-bin-native"
RPROVIDES:${PN} += "python python3 python3-core python3-misc pypy3"

inherit allarch

BBCLASSEXTEND += "native"

SOLIBS = ".so"
FILES_SOLIBSDEV = ""

INSANE_SKIP:${PN} += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-dbg += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-dev += "already-stripped file-rdeps libdir dev-elf"
INSANE_SKIP:${PN}-staticdev += "already-stripped file-rdeps libdir dev-elf"

BBCLASSEXTEND = "native"

python do_move_lic () {
    bb.build.exec_func("move_lic", d)
}

move_lic () {
    cp ${WORKDIR}/pypy3*-src/LICENSE ${S}/
}

addtask do_move_lic after do_unpack before do_populate_lic

do_compile[depends] += "pypy-2-bin-native:do_populate_sysroot"

do_compile () {
    cd ${WORKDIR}/pypy3*-src/pypy/goal
    #pypy2 ../../rpython/bin/rpython --platform arm --jit-backend arm -Ojit targetpypystandalone
    pypy2 ../../rpython/bin/rpython --platform arm -Ojit targetpypystandalone
}

do_install () {
    mkdir -p ${D}${libdir} ${D}${bindir} ${D}${includedir} ${D}/usr/local/bin/

    cp -r --no-preserve=ownership ${WORKDIR}/pypy3*-aarch64/lib/. ${D}${libdir}
    rm ${D}${libdir}/libexpat.so.1 ${D}${libdir}/libsqlite3.so.0

    cp -r --no-preserve=ownership ${WORKDIR}/pypy3*-aarch64/bin/. ${D}${bindir}
    cp -r --no-preserve=ownership ${WORKDIR}/pypy3*-aarch64/include/. ${D}${includedir}

    ln -sr ${D}${bindir}/python ${D}/usr/local/bin/python 
}
