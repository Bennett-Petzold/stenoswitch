do_install:append() {
    # Remove time syncing service entirely
    sed -i '/enable systemd-timesyncd.service/d' ${D}${libdir}/systemd/system-preset/*.preset
    rm -f ${D}${libdir}/systemd/system/systemd-timesyncd.service
}
