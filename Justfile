profile := "debug"
triple := "x86_64-unknown-uefi"
target_dir := justfile_directory() + "/target"

cargo_out := target_dir + "/" + file_stem(triple) + "/" + profile
efi := cargo_out + "/self-tests.efi"
ovmf := "/usr/share/edk2-ovmf/x64/OVMF_CODE.secboot.fd"
ovmf_vars_src := "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd"
efi_out := target_dir + "/uefi"
ovmf_vars := efi_out + "/vars.fd"
boot := efi_out + "/boot"
qmp_sock := target_dir + "/qmp.sock"
debug_out := target_dir + "/stdout.log"

release := "/etc/os-release"
cmdline := justfile_directory() + "/cmdline"
splash := "/usr/share/systemd/bootctl/splash-arch.bmp"
initrd := "/boot/initramfs-linux.img"

target := "/boot/vmlinuz-linux"

qemu_common := "\
qemu-system-x86_64 -nodefaults \
    -machine q35 -smp 2 -m 2G \
    --enable-kvm \
    -device isa-debug-exit,iosize=0x04 \
    -qmp unix:" + qmp_sock + ",server=on,wait=off \
    -drive if=pflash,format=raw,file=" + ovmf + ",readonly=on \
    -drive if=pflash,format=raw,file=" + ovmf_vars_src + ",readonly=on \
    -drive format=raw,file=fat:rw:" + boot + " \
"

# We need to ignore leaks because miri hates us
export MIRIFLAGS := "\
-Zmiri-strict-provenance \
-Zmiri-symbolic-alignment-check \
-Zmiri-isolation-error=warn-nobacktrace \
-Zmiri-tree-borrows \
"

# -Zmiri-ignore-leaks \
# -Zmiri-disable-stacked-borrows \
# -Zmiri-disable-isolation \
# -Zmiri-retag-fields \

@_default:
    {{just_executable()}} --list

@miri *args='':
    cargo +nightly miri nextest run -p nuefi {{args}}
    # cargo +nightly miri {{args}}

@test *args='':
    cargo +nightly nextest run {{args}}
    # cargo +nightly test {{args}}

@solver *args='':
    cargo +nightly rustc -p nuefi -- -Ztrait-solver=next
    # cargo +nightly rustc -p nuefi -- -Ztrait-solver=next-coherence

@doc *args='':
    cargo +nightly doc --no-deps {{args}}

@qemu: _setup
    {{qemu_common}} \
        -name self-tests \
        -nographic \
        -serial mon:stdio \
        -debugcon file:{{debug_out}}; \
    ret=$?; \
    if [ $ret -eq 69 ]; then \
        exit 0; \
    else \
        exit $ret; \
    fi

@_setup: _copy_vars
    if [ "{{profile}}" == "debug" ]; then \
        cargo build --target {{triple}} -p self-tests; \
    else \
        cargo build --target {{triple}} --profile {{profile}} -p self-tests; \
    fi

    rm -rf "{{boot}}"
    mkdir -p "{{boot}}/EFI/Boot"
    cp "{{efi}}" "{{boot}}/EFI/Boot/BootX64.efi"

@_copy_vars:
    mkdir -p "{{efi_out}}"
    cp -n "{{ovmf_vars_src}}" "{{ovmf_vars}}" &>/dev/null || true
