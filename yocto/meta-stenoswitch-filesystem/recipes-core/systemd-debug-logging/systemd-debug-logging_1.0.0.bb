LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../../software/LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

SRC_URI = "file://90-logging.conf"

FILES:${PN}:append = " /etc/systemd/system.conf.d/90-logging.conf"

inherit allarch

do_install () {
    install -D -m0644 ${WORKDIR}/90-logging.conf ${D}/etc/systemd/system.conf.d/90-logging.conf
}
