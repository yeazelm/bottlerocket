%global debug_package %{nil}

Name: %{_cross_os}rpi-efi
Version: 1.34 
Release: 1%{?dist}
Summary: Raspberry Pi 4 EFI firmware
License: BSD-3-Clause

URL: https://github.com/pftf/RPi4/releases/download/v1.34/RPi4_UEFI_Firmware_v1.34.zip
Source: RPi4_UEFI_Firmware_v1.34.zip

%description
%{summary}.

%prep
%autosetup -c RPi4_UEFI_Firmware_v1.34 -n RPi4_UEFI_Firmware_v1.34

%build


%install
# TODO actually do this right and copy to boot
# We might actually build from EDK2 but then we need to fetch from other places
# https://github.com/tianocore/edk2-platforms/tree/master/Platform/RaspberryPi/RPi4
install -d %{buildroot}/boot/efi/firmware/brcm
install -d %{buildroot}/boot/efi/overlays
install -p -m 0644 firmware/brcm/* -t %{buildroot}/boot/efi/firmware/brcm/
install -p -m 0644 overlays/miniuart-bt.dtbo -t %{buildroot}/boot/efi/overlays/
install -p -m 0644 overlays/upstream-pi4.dtbo -t %{buildroot}/boot/efi/overlays/
install -p -m 0644 RPI_EFI.fd %{buildroot}/boot/efi/
install -p -m 0644 start4.elf %{buildroot}/boot/efi/
install -p -m 0644 fixup4.dat %{buildroot}/boot/efi/
install -p -m 0644 config.txt %{buildroot}/boot/efi/
install -p -m 0644 bcm2711-rpi-400.dtb %{buildroot}/boot/efi/
install -p -m 0644 bcm2711-rpi-4-b.dtb %{buildroot}/boot/efi/
install -p -m 0644 bcm2711-rpi-cm4.dtb %{buildroot}/boot/efi/


%files
/boot/efi/firmware/brcm
/boot/efi/overlays
/boot/efi/RPI_EFI.fd
/boot/efi/start4.elf
/boot/efi/fixup4.dat
/boot/efi/config.txt
/boot/efi/bcm2711-rpi-400.dtb
/boot/efi/bcm2711-rpi-4-b.dtb
/boot/efi/bcm2711-rpi-cm4.dtb

%{_cross_attribution_file}
