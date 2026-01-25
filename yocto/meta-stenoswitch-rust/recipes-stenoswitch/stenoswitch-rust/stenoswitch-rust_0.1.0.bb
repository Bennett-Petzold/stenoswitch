LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../../software/LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

# Note: Updating the rust pushed to the image requires a new head commit.
# Uncommitted changes won't be added to the image.
SRC_URI = "git://${TOPDIR}/../..;protocol=file;usehead=1;subpath=software/rust;destsuffix=${WORKDIR}/${BP}"

SRCREV:pn-stenoswitch-rust = "${AUTOREV}"

inherit cargo_bin
inherit systemd

SYSTEMD_AUTO_ENABLE = "enable"
SYSTEMD_SERVICE:${PN}:append = " battery_control.service"
SYSTEMD_SERVICE:${PN}:append = " bluetooth_comms.service"
SYSTEMD_SERVICE:${PN}:append = " bluetooth_pin_en.service"
SYSTEMD_SERVICE:${PN}:append = " keyboard.service"
SYSTEMD_SERVICE:${PN}:append = " storage_pin_status.service"
SYSTEMD_SERVICE:${PN}:append = " translation_pin_en.service"
SYSTEMD_SERVICE:${PN}:append = " usb_charging.service"
SRC_URI:append = " file://services/"
FILES:${PN} += " ${systemd_unitdir}/system/"

DEPENDS:append = " udev"

# Temporary until I have a better way of flagging this
CARGO_BUILD_PROFILE = "dev"

do_compile[network] = "1"

do_compile:prepend() {
    export CURRENT_MONITOR_SPI="/dev/spidev0.0"
    export GPIO_CHIP="/dev/gpiochip0"
    export CURRENT_MONITOR_SEL="25"
    export SDA_PIN="2"
    export SCL_PIN="3"
    export CHG_EN="15"
    export ALERT_BATMON="4"
    export CHG_ON="18"
    export USB_ON="12"
    export BAT_ON="16"
    export STORE_ON="8"

    export ROW0="13"
    export ROW1="22"
    export ROW2="27"

    export RCOL0="19"
    export RCOL1="26"
    export RCOL2="7"
    export RCOL3="14"
    export RCOL4="17"

    export LCOL0="1"
    export LCOL1="12"
    export LCOL2="16"
    export LCOL3="20"
    export LCOL4="21"
}

do_install:append() {
    install -d ${D}/${systemd_unitdir}/system/
    install -m 0644 ${WORKDIR}/services/*.service ${D}/${systemd_unitdir}/system/
}
