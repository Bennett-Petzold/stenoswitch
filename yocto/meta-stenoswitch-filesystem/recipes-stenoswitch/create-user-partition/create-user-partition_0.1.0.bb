LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

SRC_URI = "file://create_user_partition.service"
FILES:${PN} = "${systemd_unitdir}/system/"

inherit systemd

SYSTEMD_SERVICE:${PN}:append = " create_user_partition.service"

do_install:append() {
    install -d ${D}/${systemd_unitdir}/system/
    install -m 0644 ${WORKDIR}/create_user_partition.service ${D}/${systemd_unitdir}/system/
}
