%global debug_package %{nil}

%define tornado_dir /neteye/shared/tornado/
%define lib_dir %{_libdir}/tornado/
%define bin_dir %{lib_dir}/bin
%define conf_dir %{tornado_dir}/conf/
%define data_dir %{tornado_dir}/data/
%define log_dir %{tornado_dir}/log/
%define script_dir %{_datadir}/neteye/tornado/scripts/
%define systemd_dir /usr/lib/systemd/system/
%define systemd_plugin_dir /etc/systemd/system/

%define userguide_dir /usr/share/icingaweb2/modules/%{name}/doc

%define build_target_dir target/release/

# --define 'debugbuild 1' will trigger a rustc debug build, not release
%if 0%{?debugbuild:1}
%define build_target_dir target/debug/
%endif

Name:    tornado
Version: 0.10.0
Release: 1
Summary: Tornado Package

Group:   Applications/System
License: GPLv3
Source0: %{name}.tar.gz

BuildRequires: openssl-devel
Requires: openssl-libs

# Requirements for build on NetEye 4 Machine
%if 0%{?el7}
BuildRequires: cargo
# Additionl Perl Modules for snmptrapd collector
Requires: perl(Cpanel::JSON::XS)
Requires: perl(NetSNMP::TrapReceiver)
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

# Don't Install spikes
#mkdir -p %{buildroot}/%{bin_dir}/spikes
#find src/%{build_target_dir} -maxdepth 1 -type f -executable -name 'spike_*' -exec cp -prv {} %{buildroot}/%{bin_dir}/bin/spikes/ \;

# install systemd services
mkdir -p %{buildroot}%{systemd_dir}
mkdir -p %{buildroot}%{systemd_plugin_dir}
cp -p conf/systemd/*.service %{buildroot}%{systemd_dir}
cp -rp conf/systemd/*.d %{buildroot}%{systemd_plugin_dir}

# Install data/work directories
mkdir -p %{buildroot}/%{log_dir}
mkdir -p %{buildroot}/%{data_dir}/archive/


# Install rsyslog config file
mkdir -p %{buildroot}/neteye/shared/rsyslog/conf/rsyslog.d/
cp conf/rsyslog_collector/05_tornado.conf %{buildroot}/neteye/shared/rsyslog/conf/rsyslog.d/

# Install snmptrapd script & config file
mkdir -p %{buildroot}/neteye/shared/snmptrapd/conf/conf.d/
mkdir -p %{buildroot}%{script_dir}
cp src/tornado/snmptrapd_collector/src/snmptrapd_collector.pl %{buildroot}%{script_dir}
cp conf/snmptrapd_collector/tornado.conf %{buildroot}/neteye/shared/snmptrapd/conf/conf.d/


# Install config files
mkdir -p %{buildroot}/%{conf_dir}/rules.d/
mkdir -p %{buildroot}/%{conf_dir}/collectors/icinga2/streams
mkdir -p %{buildroot}/%{conf_dir}/collectors/webhook/webhooks

cp -p conf/tornado/*_executor.toml %{buildroot}/%{conf_dir}
cp -p conf/icinga2_collector/icinga2_collector.toml %{buildroot}/%{conf_dir}/collectors/icinga2/

# install example rules, streams, webhooks
mkdir -p %{buildroot}%{lib_dir}/examples/rules/
mkdir -p %{buildroot}%{lib_dir}/examples/icinga2_collector_streams/
mkdir -p %{buildroot}%{lib_dir}/examples/webhook_collector_webhooks/

cp -p src/tornado/engine/config/rules.d/* %{buildroot}%{lib_dir}/examples/rules/
cp -p src/tornado/icinga2_collector/config/streams/* %{buildroot}%{lib_dir}/examples/icinga2_collector_streams/
cp -p src/tornado/webhook_collector/config/webhooks/* %{buildroot}%{lib_dir}/examples/webhook_collector_webhooks/

#install userguide

mkdir -p %{buildroot}%{userguide_dir}/
cp -p doc/how-to/* %{buildroot}%{userguide_dir}/

%post
# Copy example rules, streams only on first installation to avoid rpmnew/save files
if test "$1" == 1 ; then
    cp -p %{lib_dir}/examples/rules/* %{conf_dir}/rules.d/
    cp -p %{lib_dir}/examples/icinga2_collector_streams/* %{conf_dir}/collectors/icinga2/streams/
    cp -p %{lib_dir}/examples/webhook_collector_webhooks/* %{conf_dir}/collectors/webhook/webhooks/
fi

%files
%defattr(0755, root, root, 0775)
%{bin_dir}
%{script_dir}
%{_bindir}/tornado

%defattr(0660, root, root, 0770)
%dir %{tornado_dir}
%{data_dir}
%dir %{log_dir}
%dir %{lib_dir}
%{lib_dir}/examples
%dir %{conf_dir}
%dir %{conf_dir}/rules.d/
%dir %{conf_dir}/collectors/
%dir %{conf_dir}/collectors/icinga2/
%dir %{conf_dir}/collectors/icinga2/streams/
%dir %{conf_dir}/collectors/webhook/
%dir %{conf_dir}/collectors/webhook/webhooks/
%config(noreplace) %{conf_dir}/*_executor.toml
%config(noreplace) %{conf_dir}/collectors/icinga2/*.toml
%config(noreplace) /neteye/shared/rsyslog/conf/rsyslog.d/*
%config(noreplace) /neteye/shared/snmptrapd/conf/conf.d/*

%{systemd_dir}/*
%{systemd_plugin_dir}/*
%exclude %dir %{systemd_plugin_dir}/neteye.target.d

#Userguide
%defattr(0644, root, root, 0755)
%{userguide_dir}/*

%changelog
* Fri May 17 2019 Benjamin Groeber <benjamin.groeber@wuerth-phoenix.com> - 0.10.0-1
 - New Feature: API for Tornado Frontend
 - Tech. Spike: Integration of Frontend into Icingaweb2
 - Preview: Tornado Frontend
 - Added How-To for using tie Monitoring Endpoint

* Mon Apr 29 2019 Angelo Rosace <angelo.rosace@wuerth-phoenix.com> - 0.9.0-1
 - Added How-To for configuring an Snmptrapd Collector

* Mon Apr 15 2019 Benjamin Groeber <benjamin.groeber@wuerth-phoenix.com> - 0.8.0-1
 - New Feature: Simple Monitoring Endpoint on port 4748

* Fri Apr 12 2019 Benjamin Groeber <benjamin.groeber@wuerth-phoenix.com> - 0.7.0-1
 - Change: Created timestamp format changed from ISO8601 to unix epoch in milliseconds
 - Fixed: Provide Snmptrapd integration without user interaction

* Wed Mar 27 2019 Benjamin Groeber <benjamin.groeber@wuerth-phoenix.com> - 0.6.0-1
 - New Feature: Processing Tree and Pipelines
 - New Feature: Command check-config
 - Improvement: Snmptrapd Collector now is resilient against connection loss
 - Improvement: Snmptrapd now buffers metrics on connection loss
 - Improvement: Pipelines are completely parellelized and independent
 - Change: Rules are now ordered by name of the containing file
 - Change: UNIX Sockets have been deprecated in favour of more general TCP sockets

* Thu Mar 07 2019 Benjamin Groeber <benjamin.groeber@wuerth-phoenix.com> - 0.5.0-1
 - New Feature: Icinga2 API Action Executor
 - New Feature: Icinga2 Event Stream Collector
 - New Feature: Webhook Collector
 - Improvement: Systemd Services
 - Improvement: Preconfiguration on installation
 - Improvement: Integration in neteye
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

