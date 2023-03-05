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

# We need to ignore leaks because miri hates us
export MIRIFLAGS := "\
-Zmiri-strict-provenance \
-Zmiri-symbolic-alignment-check \
-Zmiri-isolation-error=warn-nobacktrace \
-Zmiri-ignore-leaks \
"

# -Zmiri-disable-stacked-borrows \
# -Zmiri-disable-isolation \
# -Zmiri-retag-fields \

@miri *args='':
    cargo +nightly miri nextest run -p nuefi {{args}}
    # cargo +nightly miri {{args}}

@test *args='':
    RUSTFLAGS="--cfg special_test" \
    cargo +nightly test {{args}}
    # cargo +nightly nextest run {{args}}

@doc *args='':
    cargo +nightly doc --no-deps {{args}}

@_default:
