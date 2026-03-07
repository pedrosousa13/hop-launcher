Name:           hopd
Version:        0.1.0
Release:        1%{?dist}
Summary:        Hop Launcher daemon providing launcher IPC methods

License:        MIT
URL:            https://github.com/pedrosousa13/hop-launcher
Source0:        https://github.com/pedrosousa13/hop-launcher/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  systemd-rpm-macros

%description
hopd serves Unix-socket IPC methods used by Hop Launcher frontends.
This package also ships the kde-hopd-query adapter helper CLI.

%prep
%autosetup -n hop-launcher-%{version}

%build
cargo build --manifest-path crates/hopd/Cargo.toml --release --locked

%install
install -Dpm0755 crates/hopd/target/release/hopd %{buildroot}%{_bindir}/hopd
install -Dpm0755 crates/hopd/target/release/kde-hopd-query %{buildroot}%{_bindir}/kde-hopd-query
install -Dpm0644 crates/hopd/debian/hopd.service %{buildroot}%{_userunitdir}/hopd.service

%files
%{_bindir}/hopd
%{_bindir}/kde-hopd-query
%{_userunitdir}/hopd.service

%changelog
* Sat Mar 07 2026 Hop Launcher Maintainers <maintainers@hop-launcher.invalid> - 0.1.0-1
- Initial RPM packaging for COPR publish flow.
