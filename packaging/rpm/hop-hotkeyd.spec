Name:           hop-hotkeyd
Version:        0.1.0
Release:        1%{?dist}
Summary:        Hop Launcher global hotkey daemon and bridge agent

License:        MIT
URL:            https://github.com/pedrosousa13/hop-launcher
Source0:        https://github.com/pedrosousa13/hop-launcher/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  systemd-rpm-macros

%description
hop-hotkeyd captures or bridges global shortcut events and sends launcher
toggle requests to the GTK control socket.

%prep
%autosetup -n hop-launcher-%{version}

%build
cargo build --manifest-path crates/hop-hotkeyd/Cargo.toml --release --locked

%install
install -Dpm0755 crates/hop-hotkeyd/target/release/hop-hotkeyd %{buildroot}%{_bindir}/hop-hotkeyd
install -Dpm0644 crates/hop-hotkeyd/debian/hop-hotkeyd.service %{buildroot}%{_userunitdir}/hop-hotkeyd.service

%files
%{_bindir}/hop-hotkeyd
%{_userunitdir}/hop-hotkeyd.service

%changelog
* Sat Mar 07 2026 Hop Launcher Maintainers <maintainers@hop-launcher.invalid> - 0.1.0-1
- Initial RPM packaging for COPR publish flow.
