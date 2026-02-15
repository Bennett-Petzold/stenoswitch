CMDLINE_MAXCPUS ?= "1"

# Minimize power usage by turning off as many cores as possible.
do_compile:append () {
    tr -d '\n' < "${WORKDIR}/cmdline.txt" > "${WORKDIR}/cmdline_ext.txt"
    #echo " cpufreq.default_governor=powersave maxcpus=${CMDLINE_MAXCPUS}" >> "${WORKDIR}/cmdline_ext.txt"
    echo " cpufreq.default_governor=powersave" >> "${WORKDIR}/cmdline_ext.txt"
    mv "${WORKDIR}/cmdline_ext.txt" "${WORKDIR}/cmdline.txt"
}
