%global debug_package %{nil}
%define release_target target/release/%{name}

Name:    tornado
Version: 0.1.0
Release: 1
Summary: Tornado Package

Group:   Applications/System
License: GPLv3
Source0: %{name}.tar.gz

%if 0%{?el7}
BuildRequires: cargo
%endif

%description
Tornado Package

%prep
%setup -c

%build
cd src
cargo build --release
cd -

%install
mkdir -p %{buildroot}/%{_bindir}
cp -pv src/%{release_target} %{buildroot}/%{_bindir}/

%files
%attr(0755, root, root) %{_bindir}/%{name}

%changelog
* Thu Oct 18 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.1.0-1
 - New Feature: Basic matching implementation via Operators
 - New Feature: Rule parsing from JSON
 - New Feature: Config parser

* Wed Sep 26 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.0.1-1
 - Initial release

