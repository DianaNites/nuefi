profile := "debug"
triple := "x86_64-unknown-uefi"
target_dir := justfile_directory() + "/target"
scratch_dir := justfile_directory() + "/scratch"
preset := scratch_dir + "/test_rust.preset"
cargo_out := target_dir + "/" + triple + "/" + profile
efi := cargo_out + "/uefi-stub.efi"
ovmf := "/usr/share/edk2-ovmf/x64/OVMF_CODE.secboot.fd"
ovmf_vars_src := "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd"
efi_out := target_dir + "/uefi"
ovmf_vars := efi_out + "/vars.fd"
boot := efi_out + "/boot"

release := "/etc/os-release"
cmdline := justfile_directory() + "/cmdline"
splash := "/usr/share/systemd/bootctl/splash-arch.bmp"
initrd := "/boot/initramfs-linux.img"

target := "/boot/vmlinuz-linux"

export MIRIFLAGS := "\
-Zmiri-strict-provenance \
-Zmiri-symbolic-alignment-check \
-Zmiri-isolation-error=warn-nobacktrace \
"
# -Zmiri-disable-stacked-borrows \
# -Zmiri-disable-isolation \
# -Zmiri-retag-fields \

@miri *args='':
    cargo +nightly miri nextest run {{args}}
    # cargo +nightly miri {{args}}

@test *args='':
    RUSTFLAGS="--cfg special_test" \
    cargo +nightly test {{args}}
    # cargo +nightly nextest run {{args}}

@doc *args='':
    cargo +nightly doc --no-deps {{args}}

@_default:

# @_default: _setup
#     qemu-system-x86_64 -nodefaults \
#         -machine q35 -smp 3 -m 2G \
#         -drive if=pflash,format=raw,file="{{ovmf}}",readonly=on \
#         -drive if=pflash,format=raw,file="{{ovmf_vars}}" \
#         -vga std \
#         --enable-kvm \
#         -drive format=raw,file=fat:rw:"{{boot}}" \
#         -serial stdio

# @build: _setup

# @debug: _setup
#     qemu-system-x86_64 -nodefaults \
#         -machine q35 -smp 3 -m 2G \
#         -drive if=pflash,format=raw,file="{{ovmf}}",readonly=on \
#         -drive if=pflash,format=raw,file="{{ovmf_vars}}" \
#         -vga std \
#         --enable-kvm \
#         -drive format=raw,file=fat:rw:"{{boot}}" \
#         -serial stdio \
#         -gdb tcp::3333

# @headless: _setup
#     qemu-system-x86_64 -nodefaults \
#         -machine q35 -smp 3 -m 2G \
#         -drive if=pflash,format=raw,file="{{ovmf}}",readonly=on \
#         -drive if=pflash,format=raw,file="{{ovmf_vars}}" \
#         -nographic \
#         --enable-kvm \
#         -drive format=raw,file=fat:rw:"{{boot}}" \
#         -serial stdio \
#         -gdb tcp::3333

# @headless2: _setup
#     qemu-system-x86_64 -nodefaults \
#         -machine q35 -smp 3 -m 2G \
#         -drive if=pflash,format=raw,file="{{ovmf}}",readonly=on \
#         -drive if=pflash,format=raw,file="{{ovmf_vars}}" \
#         -nographic \
#         --enable-kvm \
#         -drive format=raw,file=fat:rw:"{{boot}}" \
#         -serial mon:stdio \
#         -gdb tcp::3333

# @_setup: _copy_vars
#     if [ "{{profile}}" == "debug" ]; then cargo build; else cargo build --profile {{profile}}; fi
#     mkdir -p "{{boot}}/EFI/Boot"
#     # FIXME: Mkinitcpio is broken lol
#     # Just fundamentally does not work, uses incorrect fixed offsets
#     # mkinitcpio -p "{{preset}}"

#     rm -f "{{boot}}/EFI/Boot/BootX64.efi"

#     # ./create_uki.sh "{{profile}}"
#     nuki \
#         "{{efi}}" "{{boot}}/EFI/Boot/BootX64.efi" \
#         --cmdline="{{cmdline}}" \
#         --initrd="{{initrd}}" \
#         --kernel="{{target}}" \
#         --new-section=.osrel="{{release}}" \
#         --new-section=.splash="{{splash}}" \

#     test -e "{{boot}}/EFI/Boot/BootX64.efi"

#     # cp "{{efi}}" "{{boot}}/EFI/Boot/BootX64.efi"

# @_copy_vars:
#     mkdir -p "{{efi_out}}"
#     cp -n "{{ovmf_vars_src}}" "{{ovmf_vars}}"
