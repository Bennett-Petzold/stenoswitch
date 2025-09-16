FILESEXTRAPATHS:prepend := "${THISDIR}/files:"
SRC_URI:append := "file://hostapd.conf \
                   file://80-hostapd.preset"
FILES:${PN}:append = " ${libdir}/systemd/system-preset/80-hostapd.preset"

do_install:append() {
    # Replace generated version
    install -m 0444 ${WORKDIR}/hostapd.conf ${D}${sysconfdir}

    # Assumes systemd init system
    if ${@bb.utils.contains('DISTRO_FEATURES', 'systemd', 'true', 'false', d)}; then
        install -Dm 0444 ${WORKDIR}/80-hostapd.preset ${D}${libdir}/systemd/system-preset/80-hostapd.preset
    fi
}
