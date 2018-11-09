%global debug_package %{nil}
%define release_target_dir target/release/

Name:    tornado
Version: 0.2.0
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
EXECUTABLES="tornado_spike_actix tornado_spike_tokio uds_writer_collector"
for binary in $EXECUTABLES ; do
    cp -pv src/%{release_target_dir}/$binary %{buildroot}/%{_bindir}/
done

%files
%attr(0755, root, root) %{_bindir}/tornado_spike_actix
%attr(0755, root, root) %{_bindir}/tornado_spike_tokio
%attr(0755, root, root) %{_bindir}/uds_writer_collector

%changelog
* Fri Nov 09 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.2.0-1
 - New Feature: UDS Json Collector
 - New Feature: Logger Executor
 - New Feature: PoC Implementation using Actix
 - New Feature: PoC Implementation using Tokio
 - New Feature: Benchmark Tests
 - Improvement: Module level logging
 - Improvement: Enable LTO for release builds

* Fri Oct 19 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.1.0-1
 - New Feature: Basic matching implementation via Operators
 - New Feature: Rule parsing from JSON
 - New Feature: Config parser
 - New Feature: Logging

* Wed Sep 26 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.0.1-1
 - Initial release

