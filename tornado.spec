%global debug_package %{nil}
%define release_target target/release/%{name}

Name:    tornado
Version: 0.0.1
Release: 1
Summary: Tornado Package

Group:   Applications/System
License: ???
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
* Wed Sep 26 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.0.1-1
 - Initial release

