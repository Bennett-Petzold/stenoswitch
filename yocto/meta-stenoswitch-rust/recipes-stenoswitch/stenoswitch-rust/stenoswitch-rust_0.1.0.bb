LICENSE = "MPL-2.0"
LIC_FILES_CHKSUM = "file://${TOPDIR}/../../software/LICENSE.txt;md5=815ca599c9df247a0c7f619bab123dad"

#FILES:${PN} = "/user/ventoy.img"

SRC_URI = "file://rust"

inherit cargo_bin

do_compile[network] = "1"

do_adjust_loc () {
    source_dir="${WORKDIR}/${BP}"
    rm ${source_dir} || rmdir ${source_dir}
    ln -sr ${WORKDIR}/rust/ ${source_dir}
}

addtask adjust_loc after do_unpack before do_compile
