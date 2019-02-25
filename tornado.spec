%global debug_package %{nil}
%define release_target_dir target/release/
%define deploy_dir /opt/tornado/
%define conf_dir %{_sysconfdir}/tornado/

Name:    tornado
Version: 0.5.0
Release: 1
Summary: Tornado Package

Group:   Applications/System
License: GPLv3
Source0: %{name}.tar.gz

BuildRequires: openssl-devel
Requires: openssl-libs
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
mkdir -p %{buildroot}/%{deploy_dir}
# Install executables
EXECUTABLES="tornado_rsyslog_collector tornado_webhook_collector"
for binary in $EXECUTABLES ; do
    mkdir -p %{buildroot}/%{deploy_dir}/bin/
    cp -pv src/%{release_target_dir}/$binary %{buildroot}/%{deploy_dir}/bin/$binary
done
cp -pv src/%{release_target_dir}/tornado_engine %{buildroot}/%{deploy_dir}/bin/tornado

# Install spikes
mkdir -p %{buildroot}/%{deploy_dir}/bin/spikes
find src/%{release_target_dir} -maxdepth 1 -type f -executable -name 'spike_*' -exec cp -prv {} %{buildroot}/%{deploy_dir}/bin/spikes/ \;

# Install config files
mkdir -p %{buildroot}/%{conf_dir}/rules.d/

%files
%defattr(0755, root, root, 0775)
%{deploy_dir}/bin/tornado
%{deploy_dir}/bin/tornado_*_collector
%{deploy_dir}/bin/spikes/*

%defattr(0660, root, root, 0770)
%dir %{conf_dir}/rules.d/

%changelog
* Thu Feb 07 2019 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.5.0-1
 - New Feature: Webhook Collector
 - Improvement: Actions can now be generated with recursive payload
 - Spike Icinga2 Collector

* Wed Feb 06 2019 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.4.0-1
 - New Feature: Rsyslog Collector & Rsyslog 'omprog' forwarder
 - New Feature: Snmptrapd Collector & Embedded snmptrapd forwarder
 - New Feature: Script Executor
 - New Feature: Archive Executor
 - Improvement: Tornado Executable with 3 Level Configuration
 - Improvement: Nested Structures in Action Payload
 - Improvement: Support List Structures ( Arrays ), and Key Value Structures (Hashes)
 - Improvement: All dates are expected and parsed into ISO 8601
 - Spikes are now deployed in spikes subdirectory
 - Updated to Rust Edition 2018
 - Added criterion benchmarks and integrated google cpuprofiler as baseline

* Tue Nov 13 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.3.0-1
 - New Feature: Contains Operation
 - Improvement: Additional Benchmark for performance tracking

* Fri Nov 09 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.2.0-1
 - New Feature: UDS Json Collector
 - New Feature: Logger Executor
 - New Feature: PoC Implementation using Actix
 - New Feature: PoC Implementation using Tokio
 - New Feature: Benchmark Tests
 - Improvement: Module level logging
 - Improvement: Enable LTO for release builds
 - Improvement: Move up to date markdown documentation to project

* Fri Oct 19 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.1.0-1
 - New Feature: Basic matching implementation via Operators
 - New Feature: Rule parsing from JSON
 - New Feature: Config parser
 - New Feature: Logging

* Wed Sep 26 2018 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.0.1-1
 - Initial release

