FILESEXTRAPATHS:prepend := "${THISDIR}/${PN}:"
SRC_URI += "file://squashfs.cfg file://usb_gadget.cfg file://power.cfg"
PACKAGE_ARCH = "${MACHINE_ARCH}"

RPI_KERNEL_DEVICETREE_OVERLAYS:append = " overlays/spi0-1cs.dtbo"
