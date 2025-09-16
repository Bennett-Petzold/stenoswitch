LICENSE = "GPL-3.0-or-later"
LIC_FILES_CHKSUM = "file://license-ventoy.txt;md5=6a7c9eaafd980b1b8947087b8f3ffe07"

FILES:${PN} = "/user/ventoy.img"

SRC_URI = "https://github.com/ventoy/Ventoy/releases/download/v1.1.07/ventoy-1.1.07-linux.tar.gz;name=tarball \
           https://raw.githubusercontent.com/ventoy/Ventoy/refs/heads/master/License/license-ventoy.txt"
SRC_URI[tarball.sha512sum] = "a35db7eebd7b4bacf3bd96336bc8f2712a45bdfa76902de4f06c101bd0672e47e6cf73f61496988f3198aab866392c8c72a133bd9309ea5e7c1f4258be0fa4d0"
SRC_URI[license.sha512sum] = "e224a9dde83e0371f5be466c99a08b31bcf563f44013a50559d7af0535a4698ccea9df90a8adf6a6d7ffd1c459a4a968b7ad42357a9f3171b108dbc34de002b8"
SRC_URI[sha256sum] = "82fccb3ff90dc30e23bb6d9c19b0754e393ffcb3716763b7cd90cec2be8625d8"

inherit allarch

do_unpack[depends] += "xz-native:do_populate_sysroot"

python do_unpack () {
    bb.build.exec_func("min_unpack", d)
}

export DL_DIR

min_unpack () {
    cp ${DL_DIR}/license-ventoy.txt ${S}/

    TARBALL="${DL_DIR}/ventoy-${PV}-linux.tar.gz"
    IMG_LOC="./ventoy-${PV}/ventoy/ventoy.disk.img.xz"
    tar -xaOf "$TARBALL" "$IMG_LOC" | \
        xzcat > ${S}/ventoy.img
}

do_install () {
    install -D -m 0444 ${S}/ventoy.img ${D}/user/ventoy.img
}
