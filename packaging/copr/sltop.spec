Name:           sltop
Version:        0.2.0
Release:        1%{?dist}
Summary:        Lightweight real-time terminal system monitor for Linux
License:        MIT
URL:            https://github.com/Scoppiato-LAB/SLTOP
Source0:        https://github.com/Scoppiato-LAB/SLTOP/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.74
BuildRequires:  cargo
BuildRequires:  gcc

%description
sltop is an ultra-lightweight, real-time terminal system monitor designed
for Linux servers and development environments by Scoppiato LAB.

%prep
%autosetup -n SLTOP-%{version}

%build
cargo build --release

%install
rm -rf %{buildroot}
install -D -p -m 0755 target/release/sltop %{buildroot}%{_bindir}/sltop
install -D -p -m 0644 README.md %{buildroot}%{_docdir}/sltop/README.md
install -D -p -m 0644 LICENSE %{buildroot}%{_licensesdir}/sltop/LICENSE

%files
%license LICENSE
%doc README.md
%{_bindir}/sltop

%changelog
* Mon Oct 19 2026 Scoppiato LAB <info@scoppiato.it> - 0.2.0-1
- Initial Fedora RPM release for sltop
