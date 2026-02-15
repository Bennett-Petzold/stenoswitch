do_install:append() {
    # Remove time syncing service entirely
    sed -i '/enable systemd-timesyncd.service/d' ${D}${libdir}/systemd/system-preset/*.preset
    rm -f ${D}${libdir}/systemd/system/systemd-timesyncd.service

    mkdir -p ${D}${libdir}/systemd/system/getty@tty1.service.d/
    printf '%s\n%s\n%s' '[Service]' 'ExecStart=' 'ExecStart=-/sbin/agetty --noreset --noclear --autologin root - ${TERM}' > ${D}${libdir}/systemd/system/getty@tty1.service.d/autologin.conf
}
