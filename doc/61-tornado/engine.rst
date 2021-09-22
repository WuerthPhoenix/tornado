.. _tornado-engine:

Tornado Engine (Executable)
```````````````````````````

This crate contains the Tornado Engine executable code.

How It Works
++++++++++++

The Tornado Engine executable is a configuration of the engine based on
`actix <https://github.com/actix/actix>`__ and built as a portable
executable.

Structure of Tornado Engine
+++++++++++++++++++++++++++

This specific Tornado Engine executable is composed of the following
components:

-  A JSON collector
-  The engine
-  The archive executor
-  The Elasticsearch Executor
-  The foreach executor
-  The Icinga2 executor
-  The Director executor
-  The Monitoring executor
-  The logger executor
-  The script executor
-  The Smart Monitoring executor

Each component is wrapped in a dedicated actix actor.

This configuration is only one of many possible configurations. Each
component has been developed as an independent library, allowing for
greater flexibility in deciding whether and how to use it.

At the same time, there are no restrictions that force the use of the
components into the same executable. While this is the simplest way to
assemble them into a working product, the collectors and executors could
reside in their own executables and communicate with the Tornado engine
via a remote call. This can be achieved either through a direct TCP or
HTTP call, with an RPC technology (e.g., Protobuf, Flatbuffer, or
CAP'n'proto), or with a message queue system (e.g., Nats.io or Kafka) in
the middle for deploying it as a distributed system.

.. rubric:: CLI Commands and Configuration

The Tornado CLI has commands that allow you to use the functionality
provided. Running the Tornado executable without any arguments returns a
list of all available commands and global options that apply to every
command.

Tornado commands:

-  **check** : Checks that the configuration is valid.
-  **daemon** : Starts the Tornado daemon.
-  **help** : Prints the general help page, or the specific help of the
   given command.
-  **rules-upgrade** : Checks the current configuration and, if
   available, upgrades the rules structure to the most recent one.

Each CLI command provides its own help and usage information, you can
display using the ``help`` command.

For example, with this command you can show the help page and options of
``daemon``:

.. code:: bash

   ./tornado_engine help daemon

The Tornado configuration is partly based on configuration files and
partly based on command line parameters. The location of configuration
files in the file system is determined at startup based on the provided
CLI options.

Tornado global options:

-  **config-dir**: The filesystem folder from which the Tornado
   configuration is read. The default path is */etc/tornado*.
-  **rules-dir**: The folder where the Rules are saved in JSON format;
   this folder is relative to ``config_dir``. The default value is
   */rules.d/*.

The **check** command does not have any specific options.

The **daemon** command has options specified in the **tornado.daemon**
section of the *tornado.toml* configuration file.

In addition to these parameters, the following configuration entries are
available in the file *'config-dir'/tornado.toml*:

-  **logger**:

   -  **level**: The Logger level; valid values are *trace*, *debug*,
      *info*, *warn*, and *error*.
   -  **stdout**: Determines whether the Logger should print to standard
      output. Valid values are ``true`` and ``false``.
   -  **file_output_path**: A file path in the file system; if provided,
      the Logger will append any output to it.

-  **tornado.daemon**

   -  **thread_pool_config**: The configuration of the thread pools
      bound to the internal queues. This entry is optional and should be
      rarely configured manually. For more details see the following
      *Structure and Configuration: The Thread Pool Configuration*
      section.
   -  **retry_strategy.retry_policy**: The global retry policy for
      reprocessing failed actions. (Optional. Defaults to
      ``MaxAttempts`` if not provided). For more details see the
      following *Structure and Configuration: Retry Strategy
      Configuration* section.
   -  **retry_strategy.backoff_policy**: The global back-off policy for
      reprocessing failed actions. (Mandatory only if
      ``retry_strategy.retry_policy`` is provided). For more details see
      the following *Structure and Configuration: Retry Strategy
      Configuration* section.
   -  **event_tcp_socket_enabled**: Whether to enable the TCP server for
      incoming events (Optional. Valid values are ``true`` and
      ``false``. Defaults to ``true`` if not provided).
   -  **event_socket_ip**: The IP address where Tornado will listen for
      incoming events (Mandatory if ``event_tcp_socket_enabled`` is set
      to true).
   -  **event_socket_port**: The port where Tornado will listen for
      incoming events (Mandatory if ``event_tcp_socket_enabled`` is set
      to true).
   -  **nats_enabled**: Whether to connect to the NATS server (Optional.
      Valid values are ``true`` and ``false``. Defaults to ``false`` if
      not provided).
   -  **nats.client.addresses**: Array of addresses of the NATS nodes of
      a cluster. (Mandatory if ``nats_enabled`` is set to true).
   -  **nats.subject**: The NATS Subject where tornado will subscribe
      and listen for incoming events (Mandatory if ``nats_enabled`` is
      set to true).
   -  **nats.client.auth.type**: The type of authentication used to
      authenticate to NATS (Optional. Valid values are ``None`` and
      ``Tls``. Defaults to ``None`` if not provided).
   -  **nats.client.auth.path_to_pkcs12_bundle**: The path to a PKCS12
      file that will be used for authenticating to NATS (Mandatory if
      ``nats.client.auth.type`` is set to ``Tls``).
   -  **nats.client.auth.pkcs12_bundle_password**: The password to
      decrypt the provided PKCS12 file (Mandatory if
      ``nats.client.auth.type`` is set to ``Tls``).
   -  **nats.client.auth.path_to_root_certificate**: The path to a root
      certificate (in ``.pem`` format) to trust in addition to system's
      trust root. May be useful if the NATS server is not trusted by the
      system as default. (Optional, valid if ``nats.client.auth.type``
      is set to ``Tls``).
   -  **web_server_ip**: The IP address where the Tornado Web Server
      will listen for HTTP requests. This is used, for example, by the
      monitoring endpoints.
   -  **web_server_port**: The port where the Tornado Web Server will
      listen for HTTP requests.
   -  **web_max_json_payload_size**: The max JSON size in bytes accepted
      by a Tornado endpoint. (Optional. Defaults to 67108860 (i.e.
      64MB))

More information about the logger configuration is available in
section :ref:`tornado-common-logger`.

The default **config-dir** value can be customized at build time by
specifying the environment variable *TORNADO_CONFIG_DIR_DEFAULT*. For
example, this will build an executable that uses */my/custom/path* as
the default value:

.. code:: bash

   TORNADO_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 

The command-specific options should always be used after the command
name, while the global ones always precede it. An example of a full
startup command is:

.. code:: bash

   ./tornado_engine 
       --config-dir=./tornado/engine/config \
       daemon 

In this case, the CLI executes the **daemon** command that starts the
Engine with the configuration read from the *./tornado/engine/config*
directory. In addition, it will search for Filter and Rule definitions
in the *./tornado/engine/config/rules.d* directory in order to build the
processing tree.

.. rubric:: Structure and Configuration: The Thread Pool Configuration

Even if the default configuration should suit most of the use cases, in
some particular situations it could be useful to customise the size of
the internal queues used by Tornado. Tornado utilizes these queues to
process incoming events and to dispatch triggered actions.

Tornado uses a dedicated thread pool per queue; the size of each queue
is by default equal to the number of available logical CPUs.
Consequently, in case of an action of type *script*, for example,
Tornado will be able to run in parallel at max as many scripts as the
number of CPUs.

This default behaviour can be overridden by providing a custom
configuration for the thread pools size. This is achieved through the
optional **tornado_pool_config** entry in the **tornado.daemon** section
of the *Tornado.toml* configuration file.

.. rubric:: Example of how to dynamically configure the thread pool based on the available CPUs:

.. code:: toml

   [tornado.daemon]
   thread_pool_config = {type = "CPU", factor = 1.0}

In this case, the size of the thread pool will be equal to
``(number of available logical CPUs) multiplied by (factor)`` rounded to
the smallest integer greater than or equal to a number. If the resulting
value is less than *1*, then *1* will be used be default.

For example, if there are 16 available CPUs, then:

-  ``{type: "CPU", factor: 0.5}`` => thread pool size is 8
-  ``{type: "CPU", factor: 2.0}`` => thread pool size is 32

.. rubric:: Example of how to statically configure the thread pool based:

.. code:: toml

   [tornado.daemon]
   thread_pool_config = {type = "Fixed", size = 20}

In this case, the size of the thread pool is statically fixed at 20. If
the provided size is less than *1*, then *1* will be used be default.

.. rubric:: Structure and Configuration: Retry Strategy Configuration

Tornado allows the configuration of a global *retry strategy* to be
applied when the execution of an Action fails.

A *retry strategy* is composed by:

-  *retry policy*: the policy that defines whether an action execution
   should be retried after an execution failure;
-  *backoff policy*: the policy that defines the sleep time between
   retries.

Valid values for the *retry policy* are:

-  ``{type = "MaxRetries", retries = 5}`` => A predefined maximum amount
   of retry attempts. This is the default value with a retries set to
   20.
-  ``{type = "None"}`` => No retries are performed.
-  ``{type = "Infinite"}`` => The operation will be retried an infinite
   number of times. This setting must be used with extreme caution as it
   could fill the entire memory buffer preventing Tornado from
   processing incoming events.

Valid values for the *backoff policy* are:

-  ``{type = "Exponential", ms = 1000, multiplier = 2 }``: It increases
   the back off period for each retry attempt in a given set using the
   exponential function. The period to sleep on the first backoff is the
   ``ms``; the ``multiplier`` is instead used to calculate the next
   backoff interval from the last. This is the default configuration.

-  ``{type = "None"}``: No sleep time between retries. This is the
   default value.

-  ``{type = "Fixed", ms = 1000 }``: A fixed amount of milliseconds to
   sleep between each retry attempt.

-  ``{type = "Variable", ms = [1000, 5000, 10000]}``: The amount of
   milliseconds between two consecutive retry attempts.

   The time to wait after 'i' retries is specified in the vector at
   position 'i'.

   If the number of retries is bigger than the vector length, then the
   last value in the vector is used. For example:

   ``ms = [111,222,333]`` -> It waits 111 ms after the first failure,
   222 ms after the second failure and then 333 ms for all following
   failures.

.. rubric:: Example of a complete Retry Strategy configuration:


.. code:: toml

   [tornado.daemon]
   retry_strategy.retry_policy = {type = "Infinite"}
   retry_strategy.backoff_policy = {type = "Variable", ms = [1000, 5000, 10000]}

When not provided explicitly, the following default Retry Strategy is
used:

.. code:: toml

   [tornado.daemon]
   retry_strategy.retry_policy = {type = "MaxRetries", retries = 20}
   retry_strategy.backoff_policy = {type = "Exponential", ms = 1000, multiplier = 2 }

.. rubric:: Structure and Configuration: The JSON Collector

The :ref:`JSON collector <tornado-json-collectors>` embedded in
Tornado receives Events in JSON format and passes them to the matcher
engine.

There are two ways to receive an event; the first one is through a
direct TCP connection while the second one is using a Nats Cluster.
These two channels are independent and can coexist.

.. rubric:: Structure and Configuration: Enable the TCP event socket

Enabling the TCP event socket server allows Tornado to receive events
through a direct TCP connection.

The TCP event socket configuration entries are available in the
``tornado.toml`` file. Example of the TCP socket section the
``tornado.toml`` file:

.. code:: toml

   # Whether to enable the TCP listener
   event_tcp_socket_enabled = true
   # The IP address where we will listen for incoming events.
   event_socket_ip = "127.0.0.1"
   #The port where we will listen for incoming events.
   event_socket_port = 4747

In this case, Tornado will listen for incoming events on the TCP address
``127.0.0.1:4747``.

.. rubric:: Structure and Configuration: Enable the Nats connection


Enabling the Nats connection allows Tornado to receive events published
on a Nats cluster.

The Nats configuration entries are available in the ``tornado.toml``
file. Example of the Nats section the ``tornado.toml`` file:

.. code:: toml

   # Whether to connect to the NATS server
   nats_enabled = true

   # The addresses of the NATS server
   nats.client.addresses = ["127.0.0.1:4222"]
   # The NATS Subject where tornado will subscribe and listen for incoming events
   nats.subject = "tornado.events"

In this case, Tornado will connect to the "test-cluster" and listen for
incoming events published on "tornado.events" subject. Also, since
**nats.client.auth.type** is not provided, Tornado will not authenticate
to the NATS server.

At the moment, when the ``nats_enabled`` entry is set to ``true``, it is
required that the Nats server is available at Tornado startup.

.. rubric:: Structure and Configuration: Nats authentication

Available authentication types for Tornado are:

-  **None**: Tornado does not authenticate to the NATS server
-  **Tls**: Tornado authenticates to the NATS server via certificates
   with TLS

If not differently specified, Tornado will use the **None**
authentication type.

If you want instead to enable TLS authentication to the NATS server you
need something similar to the following configuration:

.. code:: toml

   # Whether to connect to the NATS server
   nats_enabled = true

   # The addresses of the NATS server
   nats.client.addresses = ["127.0.0.1:4222"]
   # The NATS Subject where tornado will subscribe and listen for incoming events
   nats.subject = "tornado.events"
   # The type of authentication used when connecting to the NATS server
   #nats.client.auth.type = "None"
   nats.client.auth.type = "Tls"
   # The path to a pkcs12 bundle file which contains the certificate and private key to authenicate to the NATS server
   nats.client.auth.path_to_pkcs12_bundle = "/path/to/pkcs12/bundle.pfx"
   # The password used to decrypt the pkcs12 bundle
   nats.client.auth.pkcs12_bundle_password = "mypwd"
   # The path to a root certificate (in .pem format) to trust in addition to system's trust root.
   # May be useful if the NATS server is not trusted by the system as default. Optional
   #nats.client.auth.path_to_root_certificate = "/path/to/root/certificate.crt.pem"

In this case Tornado will authenticate to the NATS server using the
certificate in the file specified in the field
``nats.client.auth.path_to_pkcs12_bundle``, using the password ``mypwd``
to decrypt the file.

.. rubric:: Structure and Configuration: The Matching Engine

The :ref:`matching engine <tornado-matcher-engine>` is the core of the
Tornado Engine. It receives Events from the collectors, processes them
with the configured Rules, and, in case of a match, generates the
Actions to be performed.

Two startup parameters determine the path to the processing tree
configuration:

-  *config-dir*: The filesystem folder where the Tornado configuration
   is saved; with a default value of */etc/tornado*.
-  *rules-dir*: A folder relative to the ``config_dir`` where the
   Filters and Rules are saved in JSON format; the default value is
   */rules.d/*.

For example, this command will run Tornado, load the configuration from
the ``/tornado/config`` directory, and load the processing tree JSON
files from the ``/tornado/config/rules`` directory::

   bash tornado_engine --config-dir=/tornado/config --rules-dir=/rules

The directory structure in the *rules-dir* reflects the processing tree
structure. Each subdirectory can contain either:

-  A Filter: A single JSON file with the filter details and a set of sub
   directories
-  A Ruleset: A set of JSON files with rules details

Each Rule and Filter composing the processing tree should be saved in a
separate file in JSON format. E.g.::

   /tornado/config/rules
                    |- node_0
                    |    |- 0001_rule_one.json
                    |    \- 0010_rule_two.json
                    |- node_1
                    |    |- inner_node
                    |    |    \- 0001_rule_one.json
                    |    \- filter_two.json
                    \- filter_one.json

All files must use the *json* extension; the system will ignore all
other file types.

In the above example, the processing tree composition is the following:

-  The root node is a **Filter** named "root".
-  The filter "root" has two child nodes: "node_0" and "node_1"
-  *node_0* is a **Ruleset** that contains two **Rules** called
   "rule_one" and "rule_two"
-  *node_1* is a **Filter** with a single child named "inner_node"
-  *inner_node* is a \*\ *Ruleset* with a single **Rule** called
   "rule_one"

In a ruleset, the natural alphanumeric order of the filenames determines
the execution order of the internal **Rules**, so the file ordering
corresponds to the processing order.

The **Filter** and **Ruleset** names are always derived from the parent
folder name with one exception: the root node is always named "root".

The **Rule** names are instead extracted from the JSON filenames. The
rule JSON filename is composed of two parts separated by the first '_'
(underscore) symbol. The first part determines the rule execution order,
and the second is the rule name. For example:

-  *0001_rule_one.json* -> 0001 determines the execution order,
   "rule_one" is the rule name
-  *0010_rule_two.json* -> 0010 determines the execution order,
   "rule_two" is the rule name

Because of this, we recommend that you adopt a file naming strategy that
permits easy reordering. A good approach is to always start the filename
with a number (e.g. *'number'*-*rule_name*.json) with some leading zeros
and with breaks in the number progression as shown above.

Rule names must be unique in a rule set. The are no constraints on rule
names in different rule sets.

A **Rule** is uniquely identified by the full path in the processing
tree. For example, the tree above defines the following rules:

-  root -> node_0 -> rule_one
-  root -> node_0 -> rule_two
-  root -> node_1 -> inner_node -> rule_one

In this example, the "root" node is the entry point of the processing
tree. When an **Event** arrives, the matcher will evaluate whether it
matches the filter condition; if this happens, the matcher process will
pass the **Event** to the filter's children, otherwise it will ignore
them.

More information and examples about the processing tree configuration
and runtime behavior can be found in the :ref:`matching engine
documentation <tornado-matcher-engine>`.

.. rubric:: Structure and Configuration: The Archive Executor

The :ref:`archive executor <tornado-archive-executor>` processes and
executes Actions of type "archive". This executor configuration is
specified in the ``archive_executor.toml`` file in the Tornado config
folder.

For instance, if Tornado is started with the command:

.. code:: bash

   tornado --config-dir=/tornado/config

then the configuration file's full path will be
``/tornado/config/archive_executor.toml``.

The archive_executor.toml file has the following structure:

.. code:: toml

   base_path =  "./target/tornado-log"
   default_path = "/default/file.log"
   file_cache_size = 10
   file_cache_ttl_secs = 1

   [paths]
   "one" = "/one/file.log"

More details about the meaning of each entry and how the archive
executor functions can be found in the :ref:`executor documentation
<tornado-archive-executor>`.

.. rubric:: Structure and Configuration: The Elasticsearch Executor

The :ref:`Elasticsearch executor <tornado-elasticsearch-executor>`
processes and executes Actions of type "elasticsearch". The
configuration for this executor is specified in the
``elasticsearch_executor.toml`` file into the Tornado config folder.

For instance, if Tornado is started with the command:

.. code:: bash

   tornado --config-dir=/tornado/config

then the configuration file's full path will be
``/tornado/config/elasticsearch_executor.toml``.

The elasticsearch_executor.toml has an optional ``default_auth``
section that allows to define the default authentication method to be
used with Elasticsearch. An action can override the default method by
specifying the ``auth`` payload parameter. All the authentication
types defined in :ref:`Elasticsearch executor
<tornado-elasticsearch-executor>` are supported.

In case the ``default_auth`` section is omitted, no default
authentication is available.

.. _defining-default-authentication-in-elasticsearch_executortoml:

.. rubric:: Defining default Authentication in elasticsearch_executor.toml

-  Connect without authentication:

   .. code:: toml

      [default_auth]
      type = "None"

-  Authentication with PEM certificates:

   .. code:: toml

      [default_auth]
      type = "PemCertificatePath"
      certificate_path = "/path/to/tornado/conf/certs/tornado.crt.pem"
      private_key_path = "/path/to/tornado/conf/certs/private/tornado.key.pem"
      ca_certificate_path = "/path/to/tornado/conf/certs/root-ca.crt"

More details about the executor can be found in the
:ref:`Elasticsearch executor <tornado-elasticsearch-executor>`.

.. rubric:: Structure and Configuration: The Foreach Executor


The :ref:`foreach executor <tornado-foreach-executor>` allows the
recursive executions of a set of actions with dynamic parameters.

More details about the executor can be found in the :ref:`foreach
executor <tornado-foreach-executor>`.

.. rubric:: Structure and Configuration: The Icinga2 Executor

The :ref:`Icinga2 executor <tornado-icinga-executor>` processes and
executes Actions of type "icinga2". The configuration for this
executor is specified in the ``icinga2_client_executor.toml`` file
into the Tornado config folder.

For instance, if Tornado is started with the command:

.. code:: bash

   tornado --config-dir=/tornado/config

then the configuration file's full path will be
``/tornado/config/icinga2_client_executor.toml``.

The icinga2_client_executor.toml has the following configuration
options:

-  **server_api_url**: The complete URL of the Icinga2 APIs.
-  **username**: The username used to connect to the Icinga2 APIs.
-  **password**: The password used to connect to the Icinga2 APIs.
-  **disable_ssl_verification**: If true, the client will not verify the
   SSL certificate of the Icinga2 server.
-  (**optional**) **timeout_secs**: The timeout in seconds for a call to
   the Icinga2 APIs. If not provided, it defaults to 10 seconds.

More details about the executor can be found in the :ref:`Icinga2 executor
documentation <tornado-icinga-executor>`.

.. rubric:: Structure and Configuration: The Director Executor

The :ref:`Director executor <tornado-director-executor>` processes
and executes Actions of type "director". The configuration for this
executor is specified in the ``director_client_executor.toml`` file into
the Tornado config folder.

For instance, if Tornado is started with the command:

.. code:: bash

   tornado --config-dir=/tornado/config

then the configuration file's full path will be
``/tornado/config/director_client_executor.toml``.

The director_client_executor.toml has the following configuration
options:

-  **server_api_url**: The complete URL of the Director APIs.
-  **username**: The username used to connect to the Director APIs.
-  **password**: The password used to connect to the Director APIs.
-  **disable_ssl_verification**: If true, the client will not verify the
   SSL certificate of the Director REST API server.
-  (**optional**) **timeout_secs**: The timeout in seconds for a call to
   the Icinga Director REST APIs. If not provided, it defaults to 10
   seconds.

More details about the executor can be found in the :ref:`Director
executor documentation <tornado-director-executor>`.

.. rubric:: Structure and Configuration: The Logger Executor

The :ref:`logger executor <tornado-logger-executor>` logs the whole
Action body to the standard `log <https://crates.io/crates/log>`__ at
the *info* level.

This executor has no specific configuration.

.. rubric:: Structure and Configuration: The Script Executor

The :ref:`script executor <tornado-script-executor>` processes and
executes Actions of type "script".

This executor has no specific configuration, since everything required
for script execution is contained in the Action itself as described in
the :ref:`executor documentation <tornado-script-executor>`.

.. rubric:: Structure and Configuration: The Smart Monitoring Check Result Executor

The configuration of the :ref:`smart_monitoring_check_result executor
<tornado-smartmon-check-executor>` is specified in the
``smart_monitoring_check_result.toml`` file into the Tornado config
folder.

The smart_monitoring_check_result.toml has the following configuration
options:

-  **pending_object_set_status_retries_attempts**: The number of
   attempts to perform a ``process_check_result`` for an object in
   pending state.
-  **pending_object_set_status_retries_sleep_ms**: The sleep time in ms
   between attempts to perform a process_check_result for an object in
   pending state.

The ``smart_monitoring_check_result.toml`` file is optional; if not
provided, the following default values will be used:

-  **pending_object_set_status_retries_attempts** = 5
-  **pending_object_set_status_retries_sleep_ms** = 2000

More details about the executor can be found in the
:ref:`smart_monitoring_check_result documentation
<tornado-smartmon-check-executor>`.

Tornado API
+++++++++++

The Tornado API endpoints allow to interact with a Tornado instance.

More details about the API can be found in the :ref:`Tornado backend
documentation <tornado-backend>`.

Self-Monitoring API
+++++++++++++++++++

The monitoring endpoints allow you to monitor the health of Tornado. In
the long run, they will provide information about the status,
activities, logs and metrics of a running Tornado instance.
Specifically, they will return statistics about latency, traffic, and
errors.

At this time, only a simple *ping* endpoint is available.

.. rubric:: Ping endpoint

This endpoint returns a simple message "pong - " followed by the current
date in ISO 8601 format.

Details:

-  name : **ping**
-  path : **/monitoring/ping**
-  response type: **JSON**
-  response example:

   .. code:: json

      {
        "message": "pong - 2019-04-12T10:11:31.300075398+02:00",
      }

.. _tornado-matcher-engine:

Matcher Engine
``````````````

The *tornado_engine_matcher* crate contains the core functions of the
Tornado Engine. It defines the logic for parsing Rules and Filters as
well as for matching Events.

The Matcher implementation details are :ref:`available here <tornado-matcher-details>`.

The Processing Tree
+++++++++++++++++++

The engine logic is defined by a processing tree with two types of
nodes:

-  **Filter**: A node that contains a filter definition and a set of
   child nodes
-  **Rule set**: A leaf node that contains a set of **Rules**

A full example of a processing tree is::

   root
     |- node_0
     |    |- rule_one
     |    \- rule_two
     |- node_1
     |    |- inner_node
     |    |    \- rule_one
     |    \- filter_two
     \- filter_one

All identifiers of the processing tree (i.e. rule names, filter names,
and node names) can be composed only of letters, numbers and the "_"
(underscore) character.

The configuration of the processing tree is stored on the file system in
small structures composed of directories and files in *json* format;
when the processing tree is read to be processed, the filter and rule
names are automatically inferred from the filenames--excluding the *json
extension*, and the node names from the directory names.

In the tree above, the root node is of type **Filter**. In fact, it
contains the definition of a filter named *filter_one* and has two child
nodes called *node_0* and *node_1*.

When the matcher receives an **Event**, it will first check if it
matches the *filter_one* condition; if it does, the matcher will proceed
to evaluate its child nodes. If, instead, the filter condition does not
match, the process stops and those children are ignored.

A node's children are processed independently. Thus *node_0* and
*node_1* will be processed in isolation and each of them will be unaware
of the existence and outcome of the other one. This process logic is
applied recursively to every node.

In the above processing tree, *node_0* is a rule set, so when the node
is processed, the matcher will evaluate an **Event** against each rule
to determine which one matches and what **Actions** are generated.

On the contrary, *node_1* is another **Filter**; in this case, the
matcher will check if the event verifies the filter condition in order
to decide whether to process its internal nodes.

Structure of a Filter
+++++++++++++++++++++

A **Filter** contains these properties:

-  ``filter name``: A string value representing a unique filter
   identifier. It can be composed only of letters, numbers and the "_"
   (underscore) character; it corresponds to the filename, stripped from
   its *.json* extension.
-  ``description``: A string providing a high-level description of the
   filter.
-  ``active``: A boolean value; if ``false``, the filter's children will
   be ignored.
-  ``filter``: A boolean operator that, when applied to an event,
   returns ``true`` or ``false``. This operator determines whether an
   **Event** matches the **Filter**; consequently, it determines whether
   an **Event** will be processed by the filter's inner nodes.

.. rubric:: Implicit Filters

If a **Filter** is omitted, Tornado will automatically infer an implicit
filter that passes through all **Events**. This feature allows for less
boiler-plate code when a filter is only required to blindly forward all
**Events** to the internal rule sets.

For example, if *filter_one.json* is a **Filter** that allows all
**Events** to pass through, then this processing tree::

   root
     |- node_0
     |    |- ...
     |- node_1
     |    |- ...
     \- filter_one.json

is equivalent to::

   root
     |- node_0
     |    |- ...
     \- node_1
          |- ...

Note that in the second tree we removed the *filter_one.json* file. In
this case, Tornado will automatically generate an implicit filter for
the *root* node, and all incoming **Events** will be dispatched to each
child node.

Structure of a Rule
+++++++++++++++++++

A **Rule** is composed of a set of properties, constraints and actions.

.. rubric:: Basic Properties


-  ``rule name``: A string value representing a unique rule identifier.
   It can be composed only of alphabetical characters, numbers and the
   "_" (underscore) character.
-  ``description``: A string value providing a high-level description of
   the rule.
-  ``continue``: A boolean value indicating whether to proceed with the
   event matching process if the current rule matches.
-  ``active``: A boolean value; if ``false``, the rule is ignored.

When the configuration is read from the file system, the rule name is
automatically inferred from the filename by removing the extension and
everything that precedes the first '_' (underscore) symbol. For example:

-  *0001_rule_one.json* -> 0001 determines the execution order,
   "rule_one" is the rule name
-  *0010_rule_two.json* -> 0010 determines the execution order,
   "rule_two" is the rule name

.. rubric:: Constraints


The constraint section contains the tests that determine whether or not
an event matches the rule. There are two types of constraints:

-  **WHERE**: A set of operators that when applied to an event returns
   ``true`` or ``false``
-  **WITH**: A set of regular expressions that extract values from an
   Event and associate them with named variables

An event matches a rule if and only if the WHERE clause evaluates to
``true`` and all regular expressions in the WITH clause return non-empty
values.

The following operators are available in the **WHERE** clause. Check
also the examples in the remainder of this document to see how to use
them.

-  **'contains'**: Evaluates whether the first argument contains the
   second one. It can be applied to strings, arrays, and maps. The
   operator can also be called with the alias **'contain'**.
-  **'containsIgnoreCase'**: Evaluates whether the first argument
   contains, in a case-insensitive way, the **string** passed as second
   argument. This operator can also be called with the alias
   **'containIgnoreCase'**.
-  **'equals'**: Compares any two values (including, but not limited to,
   arrays, maps) and returns whether or not they are equal. An alias for
   this operator is '**equal**'.
-  **'equalsIgnoreCase'**: Compares two strings and returns whether or
   not they are equal in a case-insensitive way. The operator can also
   be called with the alias **'equalIgnoreCase'**.
-  **'ge'**: Compares two values and returns whether the first value is
   greater than or equal to the second one. If one or both of the values
   do not exist, it returns ``false``.
-  **'gt'**: Compares two values and returns whether the first value is
   greater than the second one. If one or both of the values do not
   exist, it returns ``false``.
-  **'le'**: Compares two values and returns whether the first value is
   less than or equal to the second one. If one or both of the values do
   not exist, it returns ``false``.
-  **'lt'**: Compares two values and returns whether the first value is
   less than the second one. If one or both of the values do not exist,
   it returns ``false``.
-  **'ne'**: This is the negation of the **'equals'** operator. Compares
   two values and returns whether or not they are different. It can also
   be called with the aliases **'notEquals'** and **'notEqual'**.
-  **'regex'**: Evaluates whether a field of an event matches a given
   regular expression.
-  **'AND'**: Receives an array of operator clauses and returns ``true``
   if and only if all of them evaluate to ``true``.
-  **'OR'**: Receives an array of operator clauses and returns ``true``
   if at least one of the operators evaluates to ``true``.
-  **'NOT'**: Receives one operator clause and returns ``true`` if the
   operator clause evaluates to ``false``, while it returns ``false`` if
   the operator clause evaluates to ``true``.

We use the Rust Regex library (see its `github project home
page <https://github.com/rust-lang/regex>`__ ) to evaluate regular
expressions provided by the *WITH* clause and by the *regex* operator.
You can also refer to its `dedicated
documentation <https://docs.rs/regex>`__ for details about its features
and limitations.

.. rubric:: Actions

An Action is an operation triggered when an Event matches a Rule.

.. rubric:: Reading Event Fields

A Rule can access Event fields through the "${" and "}" delimiters. To
do so, the following conventions are defined:

-  The '.' (dot) char is used to access inner fields.
-  Keys containing dots are escaped with leading and trailing double
   quotes.
-  Double quote chars are not accepted inside a key.

For example, given the incoming event:

.. code:: json

   {
       "type": "trap",
       "created_ms": 1554130814854,
       "payload":{
           "protocol": "UDP",
           "oids": {
               "key.with.dots": "38:10:38:30.98"
           }
       }
   }

The rule can access the event's fields as follows:

-  ``${event.type}``: Returns **trap**
-  ``${event.payload.protocol}``: Returns **UDP**
-  ``${event.payload.oids."key.with.dots"}``: Returns **38:10:38:30.98**
-  ``${event.payload}``: Returns the entire payload
-  ``${event}``: Returns the entire event

.. rubric:: String interpolation

An action payload can also contain text with placeholders that Tornado
will replace at runtime. The values to be used for the substitution are
extracted from the incoming *Events* following the conventions mentioned
in the previous section; for example, using that Event definition, this
string in the action payload::

  Received a ${event.type} with protocol ${event.payload.protocol}

produces::

  *Received a trap with protocol UDP*

.. note:: Only values of type *String*, *Number*, *Boolean* and *null*
   are valid. Consequently, the interpolation will fail, and the
   action will not be executed, if the value associated with the
   placeholder extracted from the Event is an *Array*, a *Map*, or
   *undefined*.

Example of Filters
++++++++++++++++++

.. rubric:: Using a Filter to Create Independent Pipelines

We can use **Filters** to organize coherent set of **Rules** into
isolated pipelines.

In this example we will see how to create two independent pipelines, one
that receives only events with type 'email', and the other that receives
only those with type 'trapd'.

Our configuration directory will look like this:::

   rules.d
     |- email
     |    |- ruleset
     |    |     |- ... (all rules about emails here)
     |    \- only_email_filter.json
     |- trapd
     |    |- ruleset
     |    |     |- ... (all rules about trapds here)
     |    \- only_trapd_filter.json
     \- filter_all.json

This processing tree has a root filter *filter_all* that matches all
events. We have also defined two inner filters; the first,
*only_email_filter*, only matches events of type 'email'. The other,
*only_trapd_filter*, matches just events of type 'trap'.

Therefore, with this configuration, the rules defined in *email/ruleset*
receive only email events, while those in *trapd/ruleset* receive only
trapd events.

This configuration can be further simplified by removing the
*filter_all.json* file::

   rules.d
     |- email
     |    |- ruleset
     |    |     |- ... (all rules about emails here)
     |    \- only_email_filter.json
     \- trapd
          |- ruleset
          |     |- ... (all rules about trapds here)
          \- only_trapd_filter.json

In this case, in fact, Tornado will generate an implicit filter for the
root node and the runtime behavior will not change.

Below is the content of our JSON filter files.

Content of *filter_all.json* (if provided):

.. code:: json

   {
     "description": "This filter allows every event",
     "active": true
   }

Content of *only_email_filter.json*:

.. code:: json

   {
     "description": "This filter allows events of type 'email'",
     "active": true,
     "filter": {
       "type": "equals",
       "first": "${event.type}",
       "second": "email"
     }
   }

Content of *only_trapd_filter.json*:

.. code:: json

   {
     "description": "This filter allows events of type 'trapd'",
     "active": true,
     "filter": {
       "type": "equals",
       "first": "${event.type}",
       "second": "trapd"
     }
   }

Examples of Rules and operators
+++++++++++++++++++++++++++++++

.. rubric:: The 'contains' Operator

The *contains* operator is used to check whether the first argument
contains the second one.

It applies in three different situations:

-  The arguments are both strings: Returns true if the second string is
   a substring of the first one.
-  The first argument is an array: Returns true if the second argument
   is contained in the array.
-  The first argument is a map and the second is a string: Returns true
   if the second argument is an existing key in the map.

In any other case, it will return false.

Rule example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "contains",
         "first": "${event.payload.hostname}",
         "second": "linux"
       },
       "WITH": {}
     },
     "actions": []
   }

An event matches this rule if in its payload appears an entry with key
**hostname** and whose value is a string that contains **linux**.

A matching Event is:

.. code:: json

   {
       "type": "trap",
       "created_ms": 1554130814854,
       "payload":{
           "hostname": "linux-server-01"
       }
   }

.. rubric:: The 'containsIgnoreCase' Operator

The *containsIgnoreCase* operator is used to check whether the first
argument contains the **string** passed as second argument, regardless
of their capital and small letters. In other words, the arguments are
compared in a *case-insensitive* way.

It applies in three different situations:

-  The arguments are both strings: Returns true if the second string is
   a *case-insensitive substring* of the first one
-  The first argument is an array: Returns true if the array passed as
   first parameter contains a (string) element which is equal to the
   string passed as second argument, regardless of uppercase and
   lowercase letters
-  The first argument is a map: Returns true if the second argument
   contains, an existing, *case-insensitive*, key of the map

In any other case, this operator will return false.

Rule example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "containsIgnoreCase",
         "first": "${event.payload.hostname}",
         "second": "Linux"
       },
       "WITH": {}
     },
     "actions": []
   }

An event matches this rule if in its payload it has an entry with key
"hostname" and whose value is a string that contains "linux", **ignoring
the case** of the strings.

A matching Event is:

.. code:: json

   {
       "type": "trap",
       "created_ms": 1554130814854,
       "payload":{
           "hostname": "LINUX-server-01"
       }
   }

Additional values for *hostname* that match the rule include:
**linuX-SERVER-02**, **LInux-Host-12**, **Old-LiNuX-FileServer**, and so
on.

.. rubric:: The 'equals', 'ge', 'gt', 'le', 'lt' and 'ne' Operators

The *equals*, *ge*, *gt*, *le*, *lt*, *ne* operators are used to compare
two values.

All these operators can work with values of type Number, String, Bool,
null and Array.

.. warning:: Please be extremely careful when using these operators
   with numbers of type **float**. The representation of floating
   point numbers is often slightly imprecise and can lead to
   unexpected results (for example, see
   https://www.floating-point-gui.de/errors/comparison/ ).

Example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "OR",
         "operators": [
           {
             "type": "equals",
             "first": "${event.payload.value}",
             "second": 1000
           },
           {
             "type": "AND",
             "operators": [
               {
                 "type": "ge",
                 "first": "${event.payload.value}",
                 "second": 100
               },
               {
                 "type": "le",
                 "first": "${event.payload.value}",
                 "second": 200
               },
               {
                 "type": "ne",
                 "first": "${event.payload.value}",
                 "second": 150
               },
               {
                 "type": "notEquals",
                 "first": "${event.payload.value}",
                 "second": 160
               }
             ]
           },
           {
             "type": "lt",
             "first": "${event.payload.value}",
             "second": 0
           },
           {
             "type": "gt",
             "first": "${event.payload.value}",
             "second": 2000
           }
         ]
       },
       "WITH": {}
     },
     "actions": []
   }

An event matches this rule if *event.payload.value* exists and one or
more of the following conditions hold:

-  It is equal to *1000*
-  It is between *100* (inclusive) and *200* (inclusive), but not equal
   to *150* or to *160*
-  It is less than *0* (exclusive)
-  It is greater than *2000* (exclusive)

A matching Event is:

.. code:: json

   {
       "type": "email",
       "created_ms": 1554130814854,
       "payload":{
         "value": 110
       }
   }

Here are some examples showing how these operators behave:

-  ``[{"id":557}, {"one":"two"}]`` *lt* ``3``: *false* (cannot compare
   different types, e.g. here the first is an array and the second is a
   number)
-  ``{id: "one"}`` *lt* ``{id: "two"}``: *false* (maps cannot be
   compared)
-  ``[["id",557], ["one"]]`` *gt* ``[["id",555], ["two"]]``: *true*
   (elements in the array are compared recursively from left to right:
   so here "id" is first compared to "id", then 557 to 555, returning
   true before attempting to match "one" and "two")
-  ``[["id",557]]`` *gt* ``[["id",555], ["two"]]``: *true* (elements are
   compared even if the length of the arrays is not the same)
-  ``true`` *gt* ``false``: *true* (the value 'true' is evaluated as 1,
   and the value 'false' as 0; consequently, the expression is
   equivalent to "1 gt 0" which is true)
-  "twelve" *gt* "two": *false* (strings are compared lexically, and 'e'
   comes before 'o', not after it)

.. rubric:: The 'equalsIgnoreCase' Operator

The *equalsIgnoreCase* operator is used to check whether the strings
passed as arguments are equal in a *case-insensitive* way.

It applies **only if** both the first and the second arguments are
strings. In any other case, the operator will return false.

Rule example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "equalsIgnoreCase",
         "first": "${event.payload.hostname}",
         "second": "Linux"
       },
       "WITH": {}
     },
     "actions": []
   }

An event matches this rule if in its payload it has an entry with key
"hostname" and whose value is a string that is equal to "linux",
**ignoring the case** of the strings.

A matching Event is:

.. code:: json

   {
       "type": "trap",
       "created_ms": 1554130814854,
       "payload":{
           "hostname": "LINUX"
       }
   }

.. rubric:: The 'regex' Operator

The *regex* operator is used to check if a string matches a regular
expression. The evaluation is performed with the Rust Regex library (see
its `github project home page <https://github.com/rust-lang/regex>`__ )

Rule example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "regex",
         "regex": "[a-fA-F0-9]",
         "target": "${event.type}"
       },
       "WITH": {}
     },
     "actions": []
   }

An event matches this rule if its type matches the regular expression
[a-fA-F0-9].

A matching Event is:

.. code:: json

   {
       "type": "trap0",
       "created_ms": 1554130814854,
       "payload":{}
   }

.. rubric:: The 'AND', 'OR', and 'NOT' Operators

The *and* and *or* operators work on a set of operators, while the *not*
operator works on one single operator. They can be nested recursively to
define complex matching rules.

As you would expect:

-  The *and* operator evaluates to true if all inner operators match
-  The *or* operator evaluates to true if at least an inner operator
   matches
-  The *not* operator evaluates to true if the inner operator does not
   match, and evaluates to false if the inner operator matches

Example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "AND",
         "operators": [
           {
             "type": "equals",
             "first": "${event.type}",
             "second": "rsyslog"
           },
           {
             "type": "OR",
             "operators": [
               {
                 "type": "equals",
                 "first": "${event.payload.body}",
                 "second": "something"
               },
               {
                 "type": "equals",
                 "first": "${event.payload.body}",
                 "second": "other"
               }
             ]
           },
           {
             "type": "NOT",
             "operator": {
                 "type": "equals",
                 "first": "${event.payload.body}",
                 "second": "forbidden"
             }
           }
         ]
       },
       "WITH": {}
     },
     "actions": []
   }

An event matches this rule if in its payload:

-  The type is "rsyslog"
-  **AND** an entry with key *body* whose value is wither "something"
   **OR** "other"
-  **AND** an entry with key *body* is **NOT** "forbidden"

A matching Event is:

.. code:: json

   {
       "type": "rsyslog",
       "created_ms": 1554130814854,
       "payload":{
           "body": "other"
       }
   }

.. rubric:: A 'Match all Events' Rule

If the *WHERE* clause is not specified, the Rule evaluates to true for
each incoming event.

For example, this Rule generates an "archive" Action for each Event:

.. code:: json

   {
       "description": "",
       "continue": true,
       "active": true,
       "constraint": {
         "WITH": {}
       },
       "actions": [
         {
           "id": "archive",
           "payload": {
             "event": "${event}",
             "archive_type": "one"
           }
         }
       ]
   }

.. rubric:: The 'WITH' Clause

The *WITH* clause generates variables extracted from the Event based on
regular expressions. These variables can then be used to populate an
Action payload.

All variables declared by a Rule must be resolved, or else the Rule will
not be matched.

Two simple rules restrict the access and use of the extracted variables:

1. Because they are evaluated after the *WHERE* clause is parsed, any
   extracted variables declared inside the *WITH* clause are not
   accessible by the *WHERE* clause of the very same rule
2. A rule can use extracted variables declared by other rules, even in
   its *WHERE* clause, provided that:

   -  The two rules must belong to the same rule set
   -  The rule attempting to use those variables should be executed
      after the one that declares them
   -  The rule that declares the variables should also match the event

The syntax for accessing an extracted variable has the form:

**\_variables.**\ [*.RULE_NAME*].\ *VARIABLES_NAME*

If the *RULE_NAME* is omitted, the current rule name is automatically
selected.

Example:

.. code:: json

   {
     "description": "",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
             "type": "equals",
             "first": "${event.type}",
             "second": "trap"
       },
       "WITH": {
         "sensor_description": {
           "from": "${event.payload.line_5}",
           "regex": {
             "match": "(.*)",
             "group_match_idx": 0
           }
         },
         "sensor_room": {
           "from": "${event.payload.line_6}",
           "regex": {
             "match": "(.*)",
             "group_match_idx": 0
           }
         }
       }
     },
     "actions": [
       {
         "id": "nagios",
         "payload": {
           "host": "bz-outsideserverroom-sensors",
           "service": "motion_sensor_port_4",
           "status": "Critical",
           "host_ip": "${event.payload.host_ip}",
           "room": "${_variables.sensor_room}",
           "message": "${_variables.sensor_description}"
         }
       }
     ]
   }

This Rule matches only if its type is "trap" and it is possible to
extract the two variables "sensor_description" and "sensor_room" defined
in the *WITH* clause.

An Event that matches this Rule is:

.. code:: json

   {
     "type": "trap",
     "created_ms": 1554130814854,
     "payload":{
       "host_ip": "10.65.5.31",
       "line_1":  "netsensor-outside-serverroom.wp.lan",
       "line_2":  "UDP: [10.62.5.31]:161->[10.62.5.115]",
       "line_3":  "DISMAN-EVENT-MIB::sysUpTimeInstance 38:10:38:30.98",
       "line_4":  "SNMPv2-MIB::snmpTrapOID.0 SNMPv2-SMI::enterprises.14848.0.5",
       "line_5":  "SNMPv2-SMI::enterprises.14848.2.1.1.7.0 38:10:38:30.98",
       "line_6":  "SNMPv2-SMI::enterprises.14848.2.1.1.2.0 \"Outside Server Room\""
     }
   }

It will generate this Action:

.. code:: json

       {
         "id": "nagios",
         "payload": {
           "host": "bz-outsideserverroom-sensors",
           "service": "motion_sensor_port_4",
           "status": "Critical",
           "host_ip": "10.65.5.31",
           "room": "SNMPv2-SMI::enterprises.14848.2.1.1.7.0 38:10:38:30.98",
           "message": "SNMPv2-SMI::enterprises.14848.2.1.1.2.0 \"Outside Server Room\""
         }
       }

.. rubric:: The 'WITH' Clause - Configuration details

As already seen in the previous section, the *WITH* clause generates
variables extracted from the Event using regular expressions. There are
multiple ways of configuring those regexes to obtain the desired result.

Common entries to all configurations:

-  **from**: An expression that determines to which value to apply the
   extractor regex;
-  **modifiers_post**: A list of String modifiers to post-process the
   extracted value. See following section for additional details.

In addition, three parameters combined will define the behavior of an
extractor:

-  **all_matches**: whether the regex will loop through all the matches
   or only the first one will be considered. Accepted values are *true*
   and *false*. If omitted, it defaults to *false*

-  **match**, **named_match** or **single_key_match**: a string value
   representing the regex to be executed. In detail:

   -  **match** is used in case of an index-based regex,
   -  **named_match** is used when named groups are present.
   -  **single_key_match** is used to search in a map for a key that
      matches the regex. In case of a match, the extracted variable will
      be the value of the map associated with that key that matched the
      regex. This match will fail if more than one key matches the
      defined regex.

   Note that all these values are mutually exclusive.

-  **group_match_idx**: valid only in case of an index-based regex. It
   is a positive numeric value that indicates which group of the match
   has to be extracted. If omitted, an array with **all** groups is
   returned.

To show how they work and what is the produced output, from now on,
we'll use this hypotetical email body as input::

   A critical event has been received:

   STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3
   STATUS: OK HOSTNAME: MYHOST SERVICENAME: MYVALUE41231

Our objective is to extract from it information about the host status
and name, and the service name. We show how using different extractors
leads to different results.

**Option 1**

.. code:: json

   {
     "WITH": {
         "server_info": {
           "from": "${event.payload.email.body}",
           "regex": {
             "all_matches": false,
             "match": "STATUS:\\s+(.*)\\s+HOSTNAME:\\s+(.*)SERVICENAME:\\s+(.*)",
             "group_match_idx": 1
           }
         }
     }
   }

This extractor:

-  processes only the first match because **all_matches** is *false*
-  uses an index-based regex specified by **match**
-  returns the group of index **1**

In this case the output will be the string *"CRITICAL"*.

Please note that, if the *group_match_idx* was 0, it would have returned
*"STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3"* as in any
regex the group with index 0 always represents the full match.

**Option 2**

.. code:: json

   {
     "WITH": {
         "server_info": {
           "from": "${event.payload.email.body}",
           "regex": {
             "all_matches": false,
             "match": "STATUS:\\s+(.*)\\s+HOSTNAME:\\s+(.*)SERVICENAME:\\s+(.*)"
           }
         }
     }
   }

This extractor:

-  processes only the first match because **all_matches** is *false*
-  uses an index-based regex specified by **match**
-  returns an array with **all** groups of the match because
   *group_match_idx* is omitted.

In this case the output will be an array of strings::

   [
     "STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3",
     "CRITICAL",
     "MYVALUE2",
     "MYVALUE3"
   ]

**Option 3**

.. code:: json

   {
     "WITH": {
         "server_info": {
           "from": "${event.payload.email.body}",
           "regex": {
             "all_matches": true,
             "match": "STATUS:\\s+(.*)\\s+HOSTNAME:\\s+(.*)SERVICENAME:\\s+(.*)",
             "group_match_idx": 2
           }
         }
     }
   }

This extractor:

-  processes all matches because **all_matches** is *true*
-  uses an index-based regex specified by **match**
-  for each match, returns the group of index **2**

In this case the output will be an array of strings::

   [
     "MYVALUE2", <-- group of index 2 of the first match
     "MYHOST"    <-- group of index 2 of the second match
   ]

**Option 4**

.. code:: json

   {
     "WITH": {
         "server_info": {
           "from": "${event.payload.email.body}",
           "regex": {
             "all_matches": true,
             "match": "STATUS:\\s+(.*)\\s+HOSTNAME:\\s+(.*)SERVICENAME:\\s+(.*)"
           }
         }
     }
   }

This extractor:

-  processes all matches because **all_matches** is *true*
-  uses an index-based regex specified by **match**
-  for each match, returns an array with **all** groups of the match
   because *group_match_idx* is omitted.

In this case the output will be an array of arrays of strings::

   [
     [
       "STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3",
       "CRITICAL",
       "MYVALUE2",
       "MYVALUE3"
     ],
     [
       "STATUS: OK HOSTNAME: MYHOST SERVICENAME: MYVALUE41231",
       "OK",
       "MYHOST",
       "MYVALUE41231"
     ]
   ]

The inner array, in position 0, contains all the groups of the first
match while the one in position 1 contains the groups of the second
match.

**Option 5**

.. code:: json

   {
     "WITH": {
         "server_info": {
           "from": "${event.payload.email.body}",
           "regex": {
             "named_match": "STATUS:\\s+(?P<STATUS>.*)\\s+HOSTNAME:\\s+(?P<HOSTNAME>.*)SERVICENAME:\\s+(?P<SERVICENAME>.*)"
           }
         }
     }
   }

This extractor:

-  processes only the first match because **all_matches** is omitted
-  uses a regex with named groups specified by **named_match**

In this case the output is an object where the group names are the
property keys::

   {
     "STATUS": "CRITICAL",
     "HOSTNAME": "MYVALUE2",
     "SERVICENAME: "MYVALUE3"
   }

**Option 6**

.. code:: json

   {
     "WITH": {
         "server_info": {
           "from": "${event.payload.email.body}",
           "regex": {
             "all_matches": true,
             "named_match": "STATUS:\\s+(?P<STATUS>.*)\\s+HOSTNAME:\\s+(?P<HOSTNAME>.*)SERVICENAME:\\s+(?P<SERVICENAME>.*)"
           }
         }
     }
    }

This extractor:

-  processes all matches because **all_matches** is *true*
-  uses a regex with named groups specified by **named_match**

In this case the output is an array that contains one object for each
match::

   [
     {
       "STATUS": "CRITICAL",
       "HOSTNAME": "MYVALUE2",
       "SERVICENAME: "MYVALUE3"
     },
     {
       "STATUS": "OK",
       "HOSTNAME": "MYHOST",
       "SERVICENAME: "MYVALUE41231"
     },
   ]

.. rubric:: The 'WITH' Clause - Post Modifiers

The WITH clause can include a list of String modifiers to post-process
the extracted value. The available modifiers are:

-  *Lowercase*: it converts the resulting String to lower
   case. Syntax:
   
   .. code:: json

         {
             "type": "Lowercase"
         }

-  *Map*: it maps a string to another string value. Syntax:
   
   .. code:: json

        {
              "type": "Map",
              "mapping": {
                "Critical": "2",
                "Warning": "1",
                "Clear": "0",
                "Major": "2",
                "Minor": "1"
              },
              "default_value": "3"
        }

   The ``default_value`` is optional; when provided, it is used to map
   values that do not have a corresponding key in the ``mapping`` field.
   When not provided, the extractor will fail if a specific mapping is
   not found.
   
-  *ReplaceAll*: it returns a new string with all matches of a substring
   replaced by the new text; the ``find`` property is parsed as a regex
   if ``is_regex`` is true, otherwise it is evaluated as a static
   string. Syntax:

   .. code:: json

         {
             "type": "ReplaceAll",
             "find": "the string to be found",
             "replace": "to be replaced with",
             "is_regex": false 
         }

   In addition, when ``is_regex`` is true, is possible to interpolate
   the regex captured groups in the ``replace`` string, using the
   ``$<position>`` syntax, for example:
   
   .. code:: json

       {
           "type": "ReplaceAll",
           "find": "(?P<lastname>[^,\\s]+),\\s+(?P<firstname>\\S+)",
           "replace": "firstname: $2, lastname: $1",
           "is_regex": true 
       }

   Valid forms of the ``replace`` field are:

   -  extract from event: ``${events.payload.hostname_ext}``
   -  use named groups from regex: ``$digits and other``
   -  use group positions from regex: ``$1 and other``

-  *ToNumber*: it transforms the resulting String into a
   number. Syntax:
   
   .. code:: json

         {
             "type": "ToNumber"
         }

-  *Trim*: it trims the resulting String. Syntax:
   
   .. code:: json

         {
             "type": "Trim"
         }

A full example of a WITH clause using modifiers is:

.. code:: 

   {
     "WITH": {
         "server_info": {
          "from": "${event.payload.email.body}",
           "regex": {
             "all_matches": false,
             "match": "STATUS:\s+(.*)\s+HOSTNAME:\s+(.*)SERVICENAME:\s+(.*)",
             "group_match_idx": 1
           },
           "modifiers_post": [
               {
                 "type": "Lowercase"
               },
               {
                 "type": "ReplaceAll",
                 "find": "to be found",
                 "replace": "to be replaced with",
                 "is_regex": false
               },
               {
                 "type": "Trim"
               }
           ]
          }
        }
     }

This extractor has three modifiers that will be applied to the extracted
value. The modifiers are applied in the order they are declared, so the
extracted string will be transformed in lowercase, then some text
replaced, and finally, the string will be trimmed.

.. rubric:: Complete Rule Example 1

An example of a valid Rule in a JSON file is:

.. code:: json

   {
     "description": "This matches all emails containing a temperature measurement.",
     "continue": true,
     "active": true,
     "constraint": {
       "WHERE": {
         "type": "AND",
         "operators": [
           {
             "type": "equals",
             "first": "${event.type}",
             "second": "email"
           }
         ]
       },
       "WITH": {
         "temperature": {
           "from": "${event.payload.body}",
           "regex": {
             "match": "[0-9]+\\sDegrees",
             "group_match_idx": 0
           }
         }
       }
     },
     "actions": [
       {
         "id": "Logger",
         "payload": {
           "type": "${event.type}",
           "subject": "${event.payload.subject}",
           "temperature:": "The temperature is: ${_variables.temperature} degrees"
         }
       }
     ]
   }

This creates a Rule with the following characteristics:

-  Its unique name is 'emails_with_temperature'. There cannot be two
   rules with the same name.
-  An Event matches this Rule if, as specified in the *WHERE* clause, it
   has type "email", and as requested by the *WITH* clause, it is
   possible to extract the "temperature" variable from the
   "event.payload.body" with a non-null value.
-  If an Event meets the previously stated requirements, the matcher
   produces an Action with *id* "Logger" and a *payload* with the three
   entries *type*, *subject* and *temperature*.
