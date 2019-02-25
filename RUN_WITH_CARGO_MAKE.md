# Run Tornado locally with cargo-make

[cargo-make](https://github.com/sagiegurari/cargo-make) is a task runner that permits to define 
and configure sets of tasks and run them as a flow. In Tornado, it allows to start a complete 
environment, directly from the source code, for simple local testing.


## Prerequisits
To proceed with this guide, you need to install cargo-make in your system. 

In order to install it, just run the following command:
```bash
cargo install --force cargo-make
```

For more information about cargo-make functionalities and installation options, please
[refer to its documentation](https://github.com/sagiegurari/cargo-make).

## Configuration
The run configuration is in the _src/Makefile.toml_ which is the default configuration 
file used by cargo-make.

Into this file, each Tornado executable has its own run task. 
For example, the one for the Engine is `tasks.run-engine`.

The tasks define the run instructions and the command line paramenters used 
to launch the executable.


## Start Tornado Engine
To start the [tornado_engine](src/tornado/engine/doc/README.md), enter the _src_ folder and run:

```bash
cargo make run-engine
```

This builds and starts a Tornado Engine on the local machine configured to receive events 
on the UDS path _/tmp/tornado_.


## Start Tornado Webhook Collector
To start the [tornado_webhook_collector](src/tornado/webhook_collector/doc/README.md), 
enter the _src_ folder and run:

```bash
cargo make run-webhook-collector
```

This builds and starts the Tornado Webhook collector Web Servet at port 8080. 
For each incoming webhook message, it will send a Tornado Event on the UDS path _/tmp/tornado_.


## Start Tornado Rsyslog Collector
The [tornado_rsyslog_collector](src/tornado/rsyslog_collector/doc/README.md)
should be managed by the 
[omprog rsyslog module](https://www.rsyslog.com/doc/v8-stable/configuration/modules/omprog.html). 

To simplify local testing, a spike that produces fake rsyslog events was developed 
(see ./src/spike/rsyslog_collector_writer). This fake rsyslog starts the 
rsyslog_collector and forwards fake events to it. Consequently, the 
collector will generate Tornado Events and forward them to the Engine.

To start the fake rsyslog and the rsyslog_collector, enter the _src_ folder and run:

```bash
cargo make run-rsyslog-collector
```

This builds and starts both the spike and the rsyslog-collector. 
The collector will send Tornado Events on the UDS path _/tmp/tornado_.


## Start fake snmptrapd
A spike that produces fake snmptrapd events is available for local testing
(see ./src/spike/snmptrapd_collector_writer). This fake snmptrapd 
generates fake events and forwards them directly to the Engine 
on the UDS path _/tmp/tornado_snmptrapd_.

To start the fake snmptrapd , enter the _src_ folder and run:

```bash
cargo make run-snmptrapd-writer
```
