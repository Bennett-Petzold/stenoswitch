LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

RDEPENDS:${PN} += " raspi-utils"

SRC_URI = "file://disable_hdmi.service"
FILES:${PN} = "${systemd_unitdir}/system/"

inherit systemd

SYSTEMD_SERVICE:${PN}:append = " disable_hdmi.service"

do_install:append() {
    install -d ${D}/${systemd_unitdir}/system/
    install -m 0644 ${WORKDIR}/disable_hdmi.service ${D}/${systemd_unitdir}/system/
}
