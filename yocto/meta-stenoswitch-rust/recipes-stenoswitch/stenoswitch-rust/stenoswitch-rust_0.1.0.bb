LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../../software/LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

# Note: Updating the rust pushed to the image requires a new head commit.
# Uncommitted changes won't be added to the image.
SRC_URI = "git://${TOPDIR}/../..;protocol=file;usehead=1;subpath=software/rust;destsuffix=${WORKDIR}/${BP}"

SRCREV:pn-stenoswitch-rust = "${AUTOREV}"

inherit cargo_bin

do_compile[network] = "1"

do_compile:prepend() {
    export CURRENT_MONITOR_SPI=""
    export GPIO_CHIP=""
    export CURRENT_MONITOR_SEL=""
    export BATTERY_I2C=""
    export CHG_EN=""
    export ALERT_BATMON=""
    export CHG_ON=""
    export USB_ON=""
    export BAT_ON=""
}
