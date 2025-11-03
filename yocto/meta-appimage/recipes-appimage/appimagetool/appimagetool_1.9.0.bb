LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=fb9520f750ae495373b414120ab9f5c9"

FILES:${PN} = "${bindir}/appimagetool.AppImage"
SYSROOT_DIRS:append = "${bindir}/appimagetool.AppImage"

#SRC_URI = "https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-x86_64.AppImage;name=x86 \
#           https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-aarch64.AppImage;name=aarch64 \
#           https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-armhf.AppImage;name=armhf \
#           https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-i686.AppImage;name=i686 \
#           https://raw.githubusercontent.com/AppImage/appimagetool/refs/heads/main/LICENSE"
SRC_URI = "https://raw.githubusercontent.com/AppImage/appimagetool/refs/heads/main/LICENSE"
SRC_URI[license.sha512sum] = "a2116226ca651b506f266f3aec3aa0d2413c109c1ba8f0bb45c7cd0472ad13c01483cf22c1e511be0fa14138399e7768861d8e98a263ef3306e392c9478c4b50"

SRC_URI_x86:append = "https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-x86_64.AppImage;name=x86"
SRC_URI[x86.sha512sum] = "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"

BBCLASSEXTEND = "native"

do_install () {
    install -D -m 0444 ${S}/appimagetool*.AppImage ${D}/${bindir}/appimagetool.AppImage
}
