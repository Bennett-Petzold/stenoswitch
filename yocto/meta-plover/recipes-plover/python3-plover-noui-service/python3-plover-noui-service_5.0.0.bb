LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../../software/LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

inherit systemd

RDEPENDS:${PN} = "python3-plover-noui"

SYSTEMD_AUTO_ENABLE = "enable"
SYSTEMD_SERVICE:${PN} = "plover_noui.service create_plover_user_config.service translation_pin_en.service"
SRC_URI = " file://services/ file://plover-default-cfg/"
FILES:${PN} += " ${systemd_unitdir}/system/ /system/config/plover/ /root/.config/plover"

do_install:append() {
    install -d ${D}/${systemd_unitdir}/system/
    install -D -m 0644 ${WORKDIR}/services/* ${D}/${systemd_unitdir}/system/

    # Create config dir link
    install -d ${D}/root/.config/
    ln -s /user/data/.config/plover ${D}/root/.config/plover

    # Fill in system config
    install -d ${D}/system/config/plover/
    install -D -m 0444 ${WORKDIR}/plover-default-cfg/* ${D}/system/config/plover/
}
