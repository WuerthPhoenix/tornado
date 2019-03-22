# Tornado Engine (Executable)

This crate contains the Tornado Engine executable code.



## How It Works

The Tornado Engine executable is a configuration of the engine based on
[actix](https://github.com/actix/actix)
and built as a portable executable.



## Structure of Tornado Engine

This specific Tornado Engine executable is composed of the following components:
- The json collector
- The snmptrapd collector
- The engine
- The archive executor
- The Icinga2 executor
- The script executor

Each component is wrapped in a dedicated actix actor.

This configuration is only one of many possible configurations. Each component has been developed
as an independent library, allowing for greater flexibility in deciding whether and how to use it.

At the same time, there are no restrictions that force the use of the components into the same
executable. While this is the simplest way to assemble them into a working product, the
collectors and executors could reside in their own executables and communicate with the Tornado
engine via a remote call. This can be achieved either through a direct TCP or HTTP call, with
an RPC technology (e.g., Protobuf, Flatbuffer, or CAP'n'proto), or with a message queue
system (e.g., Nats.io or Kafka) in the middle for deploying it as a distributed system.



### Configuration

The configuration is partly based on configuration files and partly based on command line
parameters.

The startup parameters are:
- __logger-stdout__:  Determines whether the Logger should print to standard output.
  Valid values are `true` and `false`, with `false` the default.
- __logger-file-path__:  A file path in the file system; if provided, the Logger will
  append any output to it.
- __logger-level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
  _error_. The default value is _warn_.
- __config-dir__:  The filesystem folder from which the Tornado configuration is read.
  The default path is _/etc/tornado_.
- __rules-dir__:  The folder where the Rules are saved in JSON format;
  this folder is relative to `config_dir`. The default value is _/rules.d/_.
- __tcp-address__:  The TCP address where Tornado will listen for incoming events.
  By default it is _0.0.0.0:4747_.
- __snmptrapd-tcp-adress__:  The TCP address where Tornado will listen for incoming snmptrapd events.
  By default it is _0.0.0.0:4748_.

More information about the logger configuration is available [here](../../../common/logger/doc/README.md).

An example of a full startup command is:
```bash
./tornado_engine --logger-stdout --logger-level=debug \
    --config-dir=./tornado/engine/config \
    --tcp-address=0.0.0.0:12345 \
    --snmptrapd-tcp-address=0.0.0.0:67890
```

In this case the Engine:
- Logs to standard output at the _debug_ level
- Reads the configuration from the _./tornado/engine/config_ directory
- Searches for Rules definitions in the _./tornado/engine/config/rules.d_ directory
- Opens two TCP ports at _0.0.0.0:12345_ and _0.0.0.0:67890_ for receiving,
  respectively, the Event and Snmptrapd inputs



### Structure and Configuration: The JSON Collector

The [json collector](../../../collector/json/doc/README.md)
receives Events in JSON format and passes them to the matcher engine.

The events to be delivered to the JSON collector are published to the TCP port
configured by the _tcp-address_ command line parameter.

Example:
```bash
tornado --tcp-address=127.0.0.1:4747
```

If not specified, Tornado will use the default value `0.0.0.0:4747`.



### Structure and Configuration:  The snmptrapd Collector

The [snmptrapd collector](../../../collector/snmptrapd/doc/README.md) receives snmptrap-specific
inputs, transforms them into Tornado Events, and forwards them to the matcher engine. Snmptrapd
events are published to the TCP address configured by the _snmptrapd-tcp-address_ command line
parameter.

Example:
```bash
tornado --snmptrapd-tcp-address=127.0.0.1:4748
```

If not specified, Tornado will use the default value `0.0.0.0:4748`.

The snmptrapd input documents should be in JSON format as described by the
[snmptrapd collector's documentation](../../../collector/snmptrapd/doc/README.md).



### Structure and Configuration:  The Matching Engine

The [matching engine](../../../engine/matcher/doc/README.md) is the core of the Tornado Engine.
It receives Events from the collectors,
processes them with the configured Rules, and, in case of a match, generates the Actions to be
performed.

Two startup parameters determine the path to the Rules configuration:
- _config-dir_:  The filesystem folder where the Tornado configuration is saved;
  with a default value of _/etc/tornado_.
- _rules-dir_:  A folder relative to the `config_dir` where the Rules are saved in JSON format;
  the default value is _/rules.d/_.

For example, this command will run Tornado and load the Rules configuration from the
`/tornado/config/rules` directory:
```bash
tornado_engine --config-dir=/tornado/config --rules-dir=/rules
```

Each Rule should be saved in a separate file in the configuration directory in JSON format.
E.g.:
```
/tornado/config/rules
                 |- 0001_rule_one.json
                 |- 0010_rule_two.json
                 |- ...
```

The rule files must use the _json_ extension; the system will ignore every other file type.

The natural alphanumerical order of the filenames determines the Rules execution order at runtime.
So, the file ordering corresponds to the processing order.

Based on this, it is recommended to adopt a file naming strategy that permits easy reordering.
A good approach is to always start the filename with a number 
(e.g. _'number'_-*rule_name*.json) with some leading zeros and with holes in the number
progression as shown above.  

More information and examples about the Rule's properties and configuration can be found in the
[matching engine documentation](../../../engine/matcher/doc/README.md)



### Structure and Configuration:  The Archive Executor

The [archive executor](../../../executor/archive/doc/README.md) processes and executes Actions
of type "archive". This executor configuration is specified in the `archive_executor.toml`
file in the Tornado config folder.

For instance, if Tornado is started with the command:
```bash
tornado --config-dir=/tornado/config
```
then the configuration file's full path will be `/tornado/config/archive_executor.toml`.

The archive_executor.toml file has the following structure:
```toml
base_path =  "./target/tornado-log"
default_path = "/default/file.log"
file_cache_size = 10
file_cache_ttl_secs = 1

[paths]
"one" = "/one/file.log"
```

More details about the meaning of each entry and how the archive executor functions can be found
in the [executor documentation](../../../executor/archive/doc/README.md).



### Structure and Configuration:  The Icinga2 Executor

The [Icinga2 executor](../../../executor/icinga2/doc/README.md) processes and executes Actions
of type "icinga2". The configuration for this executor is specified in the `icinga2_client_executor.toml`
file into the Tornado config folder.

For instance, if Tornado is started with the command:
```bash
tornado --config-dir=/tornado/config
```
then the configuration file's full path will be `/tornado/config/icinga2_client_executor.toml`.

The icinga2_client_executor.toml has the following configuration options:
- __server_api_url__: The complete URL of the Icinga2 APIs.
- __username__: The username used to connect to the Icinga2 APIs.
- __password__: The password used to connect to the Icinga2 APIs.
- __disable_ssl_verification__: If true, the client will not verify the SSL certificate of the Icinga2 server.

More details about the executor can be found in the
[Icinga2 executor documentation](../../../executor/icinga2/doc/README.md).



### Structure and Configuration:  The Script Executor

The [script executor](../../../executor/script/doc/README.md) processes and executes Actions
of type "script".

This executor has no specific configuration, since everything required for script execution is
contained in the Action itself as described in the
[executor documentation](../../../executor/script/doc/README.md)
