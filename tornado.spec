%global debug_package %{nil}

%define tornado_dir /neteye/shared/tornado/
%define bin_dir %{_libdir}/tornado/bin
%define conf_dir %{tornado_dir}/conf/
#%define log_dir %{tornado_dir}/log/
%define systemd_dir /usr/lib/systemd/system/

%define build_target_dir target/release/

# --define 'debugbuild 1' will trigger a rustc debug build, not release
%if 0%{?debugbuild:1}
%define build_target_dir target/debug/
%endif

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

%if 0%{?debugbuild:1}
cargo build
%else
cargo build --release
%endif

cd -

%install
mkdir -p %{buildroot}/%{bin_dir}
mkdir -p %{buildroot}/%{conf_dir}
#mkdir -p %{buildroot}/%{log_dir}
mkdir -p %{buildroot}/%{_bindir}

# Install executables
cp -pv src/%{build_target_dir}/tornado_*_collector %{buildroot}/%{bin_dir}

#EXECUTABLES="tornado_rsyslog_collector tornado_webhook_collector"
#for binary in $EXECUTABLES ; do
#    mkdir -p %{buildroot}/%{bin_dir}
#    cp -pv src/%{build_target_dir}/$binary %{buildroot}/%{bin_dir}/$binary
#done

# Install tornado daemon
cp -pv src/%{build_target_dir}/tornado_engine %{buildroot}%{_bindir}/tornado

# Install spikes
mkdir -p %{buildroot}/%{bin_dir}/spikes
find src/%{build_target_dir} -maxdepth 1 -type f -executable -name 'spike_*' -exec cp -prv {} %{buildroot}/%{bin_dir}/bin/spikes/ \;

# Install config files
mkdir -p %{buildroot}/%{conf_dir}/rules.d/

# install systemd services
mkdir -p %{buildroot}%{systemd_dir}
cp conf/systemd/tornado.service %{buildroot}%{systemd_dir}

%files
%defattr(0755, root, root, 0775)
%{bin_dir}
%{_bindir}/tornado

%defattr(0660, root, root, 0770)
%{conf_dir}
%{systemd_dir}/*
#%{log_dir}

%changelog
* Thu Feb 07 2019 Benjamin Groeber <Benjamin.Groeber@wuerth-phoenix.com> - 0.5.0-1
 - New Feature: Icinga2 API Action Executor
 - New Feature: Icinga2 Event Stream Collector
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

