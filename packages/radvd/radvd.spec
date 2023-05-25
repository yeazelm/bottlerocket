Name: %{_cross_os}radvd
Version: 2.19
Release: 1%{?dist}
Summary: A Router Advertisement daemon
License: BSD with advertising
URL: http://www.litech.org/radvd/
Source0: %{url}dist/radvd-%{version}.tar.gz
Source1: radvdump.service 

BuildRequires: bison
BuildRequires: flex
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n radvd-%{version} -p1

%build
%cross_configure

%make_build

%install
%make_install
install -d %{buildroot}%{_cross_unitdir}
install -m 644 %{S:1} %{buildroot}%{_cross_unitdir}

#install -m 755 radvd %{buildroot}%{_cross_sbindir}


%files
%license COPYRIGHT
%{_cross_attribution_file}
#%{_cross_sbindir}/radvd
#%{_cross_sbindir}/radvdump
%{_cross_unitdir}/radvdump.service

%changelog
