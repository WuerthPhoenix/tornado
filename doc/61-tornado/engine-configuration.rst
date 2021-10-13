.. _tornado-engine-conf:

Tornado Engine CLI Commands and Configuration
`````````````````````````````````````````````

The Tornado CLI has commands that allow you to use the functionality
provided. Running the Tornado executable without any arguments returns a
list of all available commands and global options that apply to every
command.

Tornado commands:

- __apm-tracing enable|disable__ : Enable or disable the APM priority logger output.

  When used with `enable`, it:

  - enables the APM logger
  - disables the stdout logger output
  - sets logger level to `info,tornado=debug`

  When used with `disable`, it:

  - disables the APM logger
  - enables the stdout logger output
  - sets logger level to value from the configuration file

-  **check** : Checks that the configuration is valid.
-  **daemon** : Starts the Tornado daemon.
-  **help** : Prints the general help page, or the specific help of the
   given command.
-  **rules-upgrade** : Checks the current configuration and, if
   available, upgrades the rules structure to the most recent one.
-  **filter-create** : Creates a filter at the root level of the current
   configuration and of the open drafts.

   Parameters:

      - **--name**: The name of the filter to be created.
      - **--json-definition**: The JSON representation of the filter.

   In case a Node with the same name already exists at the root level of the
   configuration, the following will happen:

      - If the existing node is a Ruleset, it will be renamed to: `<ruleset_name>_backup_<timestamp_in_milliseconds>`.

      - If the existing node is a Filter having a different `filter` definition,
        it will be renamed to: `<filter_name>_backup_<timestamp_in_milliseconds>`.

      - If the existing node is a Filter having the same `filter` definition,
        nothing will be done.


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
   -  **nats_extractors**: List of data extractors for incoming Nats messages (Optional).
      Valid extractors are:

        - **FromSubject**: using a regex, extracts the first matching group from the Nats subject and
          adds its value to the *event.metadata* scope using the specified key. Example:

          .. code:: toml

             nats_extractors = [
                 { type = "FromSubject", key = "tenant_id", regex = "^([^.]+)\\.tornado\\.events" }
             ]

   -  **nats.client.addresses**: Array of addresses of the NATS nodes of
      a cluster. (Mandatory if ``nats_enabled`` is set to true).
   -  **nats.subject**: The NATS Subject where tornado will subscribe
      and listen for incoming events (Mandatory if ``nats_enabled`` is
      set to true).
   -  **nats.client.auth.type**: The type of authentication used to
      authenticate to NATS (Optional. Valid values are ``None`` and
      ``Tls``. Defaults to ``None`` if not provided).
   -  **nats.client.auth.certificate_path**: The path to the client
      certificate that will be used for authenticating to NATS.
      (Mandatory if `nats.client.auth.type` is set to `Tls`).
   -  **nats.client.auth.private_key_path**: The path to the client
      certificate private key that will be used for authenticating to
      NATS.  (Mandatory if `nats.client.auth.type` is set to `Tls`).
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
