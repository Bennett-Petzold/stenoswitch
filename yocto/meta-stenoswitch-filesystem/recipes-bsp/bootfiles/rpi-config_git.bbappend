do_deploy:append () {
    printf '%s\n' 'dtoverlay=spi0-1cs,cs0_pin=25' >> "${CONFIG}"
}
