ETH_LED_OFF ??= "14"

do_deploy:append () {
    # TODO add dtoverlay=disable-wifi on non-debug builds

    printf '%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n' \
        'dtparam=act_led_trigger=timer' \
        'dtparam=audio=off' \
        'dtparam=axiperf=off' \
        "dtparam=eth_led0=${ETH_LED_OFF}" \
        "dtparam=eth_led1=${ETH_LED_OFF}" \
        'dtparam=hdmi=off' \
        'dtparam=spi=on' \
        'dtparam=uart0=off' \
        'dtparam=uart1=off' \
        'dtoverlay=i2c_arm=off' \
        'dtoverlay=i2c_csi_dsi=off' \
        'dtoverlay=spi0-1cs,cs0_pin=25' \
        'dtoverlay=dwc2,dr_mode=peripheral' \
    >> "${CONFIG}"
}
