%global debug_package %{nil}

Name: %{_cross_os}brcm-firmware
Version: 0.1
Release: 1%{?dist}
Summary: Install Broadcom firmware
License: ASL
# Note: You can see changes here:

URL: file:///home/yeazelm/git/bottlerocket/packages/brcm-firmware/brcm-firmware.tgz
Source: brcm-firmware.tgz

%description
%{summary}.

%prep
%autosetup -n firmware -p1

%build


%install
mkdir -p  %{buildroot}%{_cross_libdir}/firmware/brcm
install -p -m 0644 brcm/* -t %{buildroot}%{_cross_libdir}/firmware/brcm/

%files
%dir %{_cross_libdir}/firmware
%dir %{_cross_libdir}/firmware/brcm
%{_cross_libdir}/firmware/brcm/*
%{_cross_attribution_file}
