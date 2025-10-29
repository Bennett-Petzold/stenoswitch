FILESEXTRAPATHS:prepend := "${THISDIR}/files:"

FILES:${PN}:append := " /user/data/"

do_install:append () {
    mkdir -p ${D}/user/data/
}
