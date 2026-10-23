# FlashCraft RPM spec.
# Repackages the released binary tree tarball (packaging/linux/tarball.sh).
# Build: packaging/linux/rpm.sh
%global debug_package %{nil}
Name:           flashcraft
Version:        0.2.0
Release:        1%{?dist}
Summary:        Write bootable images to removable drives
License:        GPL-3.0-only
URL:            https://github.com/dorit33/flashcraft
Source0:        %{name}-%{version}-linux-%{_arch}.tar.gz

BuildArch:      %{_arch}
AutoReqProv:    no
Requires:       gtk3, libxcb, libxkbcommon, mesa-libgbm, fontconfig

%description
FlashCraft writes bootable images (ISO 9660, disk images) to removable
drives, byte-for-byte, with read-back verification. A clean-room,
pure-Rust reimplementation of Rufus: native egui, no Electron, no web view.
Only removable, unmounted devices can be written — the safety is
structural, not a warning you can click through.

%prep
%setup -q -n %{name}-%{version}-linux-%{_arch}

%build
# prebuilt payload; nothing to compile in the rpm build

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/metainfo
cp -a bin/flashcraft bin/flashcraft-cli %{buildroot}%{_bindir}/
cp -a share/applications/dev.flashcraft.app.desktop %{buildroot}%{_datadir}/applications/
cp -a share/metainfo/dev.flashcraft.app.metainfo.xml %{buildroot}%{_datadir}/metainfo/
if [ -d share/icons ]; then
    mkdir -p %{buildroot}%{_datadir}/icons
    cp -a share/icons/hicolor %{buildroot}%{_datadir}/icons/
fi
if [ -d share/doc/flashcraft ]; then
    mkdir -p %{buildroot}%{_docdir}/flashcraft
    cp -a share/doc/flashcraft/* %{buildroot}%{_docdir}/flashcraft/
fi

%files
%{_bindir}/flashcraft
%{_bindir}/flashcraft-cli
%{_datadir}/applications/dev.flashcraft.app.desktop
%{_datadir}/metainfo/dev.flashcraft.app.metainfo.xml
%{_datadir}/icons/hicolor/*/apps/dev.flashcraft.app.*
%license %{_docdir}/flashcraft/LICENSE
%doc %{_docdir}/flashcraft/ATTRIBUTION.md

%changelog
* Sun Oct 18 2026 FlashCraft contributors <noreply@localhost> - 0.1.0-1
- Initial package: list/inspect/write/verify, CLI + egui desktop app.
