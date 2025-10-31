FILESEXTRAPATHS:prepend := "${THISDIR}/${PN}:"
SRC_URI += "file://squashfs.cfg"
PACKAGE_ARCH = "${MACHINE_ARCH}"

#SRC_URI:append:rpi = " file://stenoswitch-spi-pins-overlay.dts;subdir=git/arch/${ARCH}/boot/dts/overlays"
#RPI_KERNEL_DEVICETREE_OVERLAYS:append = " overlays/stenoswitch-spi-pins-overlay.dtbo"
RPI_KERNEL_DEVICETREE_OVERLAYS:append = " overlays/spi0-1cs.dtbo"
