# Tornado Engine (Executable)

This crate contains the Tornado Engine executable code.



## How It Works

The Tornado Engine executable is a configuration of the engine based on
[actix](https://github.com/actix/actix)
and built as a portable executable.



## Structure of Tornado Engine

This specific Tornado Engine executable is composed of the following components:
- A JSON collector
- The engine
- The archive executor
- The Elasticsearch Executor
- The foreach executor
- The Icinga2 executor
- The logger executor
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



### CLI Commands and Configuration

The Tornado CLI has commands that allow you to use the functionality provided.
Running the Tornado executable without any arguments returns a list of all available
commands and global options that apply to every command.

Tornado commands:
- __check__ : Checks that the configuration is valid.
- __daemon__ : Starts the Tornado daemon.
- __help__ : Prints the general help page, or the specific help of the given command.

Each CLI command provides its own help and usage information, you can display using the `help` command.

For example, with this command you can show the help page and options of `daemon`:
```bash
./tornado_engine help daemon
```

The Tornado configuration is partly based on configuration files and partly based on command line
parameters. The location of configuration files in the file system is determined at startup based
on the provided CLI options.

Tornado global options:
- __config-dir__:  The filesystem folder from which the Tornado configuration is read.
  The default path is _/etc/tornado_.
- __rules-dir__:  The folder where the Rules are saved in JSON format;
  this folder is relative to `config_dir`. The default value is _/rules.d/_.

The __check__ command does not have any specific options.

The __daemon__ command has options specified in the **tornado.daemon** section of the 
_tornado.toml_ configuration file. 

In addition to these parameters, the following configuration entries are available in the 
file _'config-dir'/tornado.toml_:
- __logger__:
    - __level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
      _error_.
    - __stdout__:  Determines whether the Logger should print to standard output.
      Valid values are `true` and `false`.
    - **file_output_path**:  A file path in the file system; if provided, the Logger will
      append any output to it.
- **tornado.daemon**
    - **event_tcp_socket_enabled**: Whether to enable the TCP server for incoming events
      (Optional. Valid values are `true` and `false`. Defaults to `true` if not provided).
    - **event_socket_ip**:  The IP address where Tornado will listen for incoming events 
    (Mandatory if `event_tcp_socket_enabled` is set to true).
    - **event_socket_port**:  The port where Tornado will listen for incoming events
    (Mandatory if `event_tcp_socket_enabled` is set to true).
    - **nats_enabled**: Whether to connect to the NATS server
    (Optional. Valid values are `true` and `false`. Defaults to `false` if not provided).
    - **nats.client.addresses**: Array of addresses of the NATS nodes of a cluster.
    (Mandatory if `nats_enabled` is set to true).
    - **nats.subject**:  The NATS Subject where tornado will subscribe and listen for incoming events
    (Mandatory if `nats_enabled` is set to true).
    - **web_server_ip**: The IP address where the Tornado Web Server will listen for HTTP requests.
      This is used, for example, by the monitoring endpoints.
    - **web_server_port**:  The port where the Tornado Web Server will listen for HTTP requests.


More information about the logger configuration is available [here](../../common/logger/README.md).

The default __config-dir__ value can be customized at build time by specifying
the environment variable *TORNADO_CONFIG_DIR_DEFAULT*. 
For example, this will build an executable that uses */my/custom/path* 
as the default value:
```bash
TORNADO_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 
```

The command-specific options should always be used after the command name, while the global ones
always precede it.  An example of a full startup command is:
```bash
./tornado_engine 
    --config-dir=./tornado/engine/config \
    daemon 
```

In this case, the CLI executes the __daemon__ command that starts the Engine with the
configuration read from the _./tornado/engine/config_ directory. In addition, 
it will search for Filter and Rule definitions in the _./tornado/engine/config/rules.d_ 
directory in order to build the processing tree.


### Structure and Configuration: The JSON Collector

The [JSON collector](../../collector/json/README.md) embedded in Tornado
receives Events in JSON format and passes them to the matcher engine.

There are two ways to receive an event; 
the first one is through a direct TCP connection while the second one is using a Nats Cluster.
These two channels are independent and can coexist.

### Structure and Configuration: Enable the TCP event socket
Enabling the TCP event socket server allows Tornado to receive events through a direct TCP connection.

The TCP event socket configuration entries are available in the `tornado.toml` file.
Example of the TCP socket section the `tornado.toml` file:
```toml
# Whether to enable the TCP listener
event_tcp_socket_enabled = true
# The IP address where we will listen for incoming events.
event_socket_ip = "127.0.0.1"
#The port where we will listen for incoming events.
event_socket_port = 4747
```

In this case, Tornado will listen for incoming events on the TCP address `127.0.0.1:4747`.


### Structure and Configuration: Enable the Nats connection
Enabling the Nats connection allows Tornado to receive events published on a Nats cluster.

The Nats configuration entries are available in the `tornado.toml` file.
Example of the Nats section the `tornado.toml` file:
```toml
# Whether to connect to the NATS server
nats_enabled = true

# The addresses of the NATS server
nats.client.addresses = ["127.0.0.1:4222"]
# The NATS Subject where tornado will subscribe and listen for incoming events
nats.subject = "tornado.events"
```

In this case, Tornado will connect to the "test-cluster" and listen for incoming events published on "tornado.events" subject.

At the moment, when the `nats_enabled` entry is set to `true`, it is required that the Nats
server is available at Tornado startup.

### Structure and Configuration:  The Matching Engine

The [matching engine](../../engine/matcher/README.md) is the core of the Tornado Engine.
It receives Events from the collectors, processes them with the configured Rules, and, in case
of a match, generates the Actions to be performed.

Two startup parameters determine the path to the processing tree configuration:
- _config-dir_:  The filesystem folder where the Tornado configuration is saved;
  with a default value of _/etc/tornado_.
- _rules-dir_:  A folder relative to the `config_dir` where the Filters and Rules are saved in
  JSON format; the default value is _/rules.d/_.

For example, this command will run Tornado, load the configuration from the `/tornado/config`
directory, and load the processing tree JSON files from the `/tornado/config/rules` directory:
```
bash tornado_engine --config-dir=/tornado/config --rules-dir=/rules
```

The directory structure in the _rules-dir_ reflects the processing tree structure. Each
subdirectory can contain either:
- A Filter:  A single JSON file with the filter details and a set of sub directories
- A Ruleset:  A set of JSON files with rules details

Each Rule and Filter composing the processing tree should be saved in a separate file in JSON format.

E.g.:
```
/tornado/config/rules
                 |- node_0
                 |    |- 0001_rule_one.json
                 |    \- 0010_rule_two.json
                 |- node_1
                 |    |- inner_node
                 |    |    \- 0001_rule_one.json
                 |    \- filter_two.json
                 \- filter_one.json
```

All files must use the _json_ extension; the system will ignore all other file types.

In the above example, the processing tree composition is the following:
- The root node is a **Filter** named "root". 
- The filter "root" has two child nodes: "node_0" and "node_1"
- *node_0* is a **Ruleset** that contains two **Rules** called "rule_one" and "rule_two"
- *node_1* is a **Filter** with a single child named "inner_node"
- *inner_node* is a **Ruleset* with a single **Rule** called "rule_one"

In a ruleset, the natural alphanumeric order of the filenames determines the execution
order of the internal __Rules__, so the file ordering corresponds to the processing order.

The __Filter__ and **Ruleset** names are always derived from the parent folder name 
with one exception:  the root node is always named "root".

The **Rule** names are instead extracted from the JSON filenames. 
The rule JSON filename is composed of two parts separated by the first '_' (underscore) symbol.
The first part determines the rule execution order, and the second is the rule name.
For example:
- _0001_rule_one.json_ -> 0001 determines the execution order, "rule_one" is the rule name
- _0010_rule_two.json_ -> 0010 determines the execution order, "rule_two" is the rule name

Because of this, we recommend that you adopt a file naming strategy that permits easy reordering.
A good approach is to always start the filename with a number (e.g. _'number'_-*rule_name*.json)
with some leading zeros and with breaks in the number progression as shown above.  

Rule names must be unique in a rule set. The are no constraints on rule names in different
rule sets.

A __Rule__ is uniquely identified by the full path in the processing tree. For example, the tree
above defines the following rules:
- root -> node_0 -> rule_one
- root -> node_0 -> rule_two
- root -> node_1 -> inner_node -> rule_one

In this example, the "root" node is the entry point of the processing tree. When an
__Event__ arrives, the matcher will evaluate whether it matches the filter condition; if this
happens, the matcher process will pass the __Event__ to the filter's children, otherwise it
will ignore them.

More information and examples about the processing tree configuration and runtime behavior can
be found in the [matching engine documentation](../../engine/matcher/README.md)



### Structure and Configuration:  The Archive Executor

The [archive executor](../../executor/archive/README.md) processes and executes Actions
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
in the [executor documentation](../../executor/archive/README.md).


### Structure and Configuration:  The Elasticsearch Executor

The [Elasticsearch executor](../../executor/elasticsearch/README.md) processes and executes Actions
of type "elasticsearch". The configuration for this executor is specified in the `elasticsearch_executor.toml`
file into the Tornado config folder.

For instance, if Tornado is started with the command:
```bash
tornado --config-dir=/tornado/config
```
then the configuration file's full path will be `/tornado/config/elasticsearch_executor.toml`.

The elasticsearch_executor.toml has an optional `default_auth` section that allows to define the default 
authentication method to be used with Elasticsearch. An action can override the default method by 
specifying the `auth` payload parameter. 
All the authentication types defined in [Elasticsearch executor](../../executor/elasticsearch/README.md)
are supported.

In case the `default_auth` section is omitted, no default authentication is available.

#### Defining default Authentication in elasticsearch_execute.toml
* Connect without authentication:      
    ```toml
    [default_auth]
    type = "None"
    ```              

* Authentication with PEM certificates:
    ```toml
    [default_auth]
    type = "PemCertificatePath"
    certificate_path = "/path/to/tornado/conf/certs/tornado.crt.pem"
    private_key_path = "/path/to/tornado/conf/certs/private/tornado.key.pem"
    ca_certificate_path = "/path/to/tornado/conf/certs/root-ca.crt"
    ```        

More details about the executor can be found in the
[Elasticsearch executor](../../executor/elasticsearch/README.md).


### Structure and Configuration:  The Foreach Executor

The [foreach executor](../../executor/foreach/README.md) allows
the recursive executions of a set of actions with dynamic parameters.

More details about the executor can be found in the
[foreach executor documentation](../../executor/foreach/README.md).


### Structure and Configuration:  The Icinga2 Executor

The [Icinga2 executor](../../executor/icinga2/README.md) processes and executes Actions
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
[Icinga2 executor documentation](../../executor/icinga2/README.md).



### Structure and Configuration:  The Logger Executor

The [logger executor](../../executor/logger/README.md) logs the whole Action body
to the standard [log](https://crates.io/crates/log) at the _info_ level.

This executor has no specific configuration.


### Structure and Configuration:  The Script Executor

The [script executor](../../executor/script/README.md) processes and executes Actions
of type "script".

This executor has no specific configuration, since everything required for script execution is
contained in the Action itself as described in the
[executor documentation](../../executor/script/README.md)



## Tornado API
The Tornado API endpoints allow to interact with a Tornado instance.

More details about the API can be found in the
[Tornado backend documentation](../engine_api/README.md).


## Self-Monitoring API

The monitoring endpoints allow you to monitor the health of Tornado.
In the long run, they will provide information about the status, activities, logs and metrics
of a running Tornado instance. Specifically, they will return statistics about
latency, traffic, and errors.

At this time, only a simple _ping_ endpoint is available.



### Ping endpoint 

This endpoint returns a simple message "pong - " followed by the current date in ISO 8601 format.

Details:
- name : __ping__
- path : __/monitoring/ping__
- response type: __JSON__ 
- response example:
  ```json
  {
    "message": "pong - 2019-04-12T10:11:31.300075398+02:00",
  }
  ```
