.. _tornado-jmespath-collector-conf:

JMESPath Collector Configuration
++++++++++++++++++++++++++++++++

The Collector configuration is composed of two named values:

-  **event_type**: Identifies the type of Event, and can be a String or
   a JMESPath expression (see below).
-  **payload**: A Map<String, ValueProcessor> with event-specific data.

and here the payload **ValueProcessor** can be one of:

-  A **null** value
-  A **string**
-  A **bool** value (i.e., true or false)
-  A **number**
-  An **array** of values
-  A **map** of type Map<String, ValueProcessor>
-  A **JMESPath expression** : A valid JMESPath expression delimited by
   the '${' and '}' characters.

The Collector configuration defines the structure of the Event produced.
The configuration's *event_type* property will define the type of Event,
while the Event's *payload* will have the same structure as the
configuration's payload.

.. rubric:: How it Works

The **JMESPath expressions** of the configuration will be applied to
incoming inputs, and the results will be included in the Event produced.
All other **ValueProcessors**, instead, are copied without modification.

For example, consider the following configuration:

.. code:: json

   {
       "event_type": "webhook",
       "payload": {
           "name" : "${reference.authors[0]}",
           "from": "jmespath-collector",
           "active": true
       }
   }

The value *${reference.authors[0]}* is a JMESPath expression, delimited
by ``${`` and ``}``, and whose value depends on the incoming input.

Thus if this input is received:

.. code:: json

   {
       "date": "today",
       "reference": {
           "authors" : [
             "Francesco",
             "Thomas"
           ]
       }
   }

then the Collector will produce this Event:

.. code:: json

   {
       "event_type": "webhook",
       "payload": {
           "name" : "Francesco",
           "from": "jmespath-collector",
           "active": true
       }
   }

.. rubric:: Runtime behavior

When the JMESPath expression returns an array or a map, the entire
object will be inserted as-is into the Event.

However, if a JMESPath expression does not return a valid result, then
no Event is created, and an error is produced.

.. _tornado-email-collector-conf:

Email Collector Configuration
+++++++++++++++++++++++++++++

The executable configuration is based partially on configuration files,
and partially on command line parameters.

The available startup parameters are:

-  **config-dir**: The filesystem folder from which the collector
   configuration is read. The default path is
   */etc/tornado_email_collector/*.

In addition to these parameters, the following configuration entries are
available in the file *'config-dir'/email_collector.toml*:

-  **logger**:

   -  **level**: The Logger level; valid values are *trace*, *debug*,
      *info*, *warn*, and *error*.
   -  **stdout**: Determines whether the Logger should print to standard
      output. Valid values are ``true`` and ``false``.
   -  **file_output_path**: A file path in the file system; if provided,
      the Logger will append any output to it.

-  **email_collector**:

   -  **tornado_event_socket_ip**: The IP address where outgoing events
      will be written. This should be the address where the Tornado
      Engine listens for incoming events. If present, this value
      overrides what specified by the ``tornado_connection_channel``
      entry. *This entry is deprecated and will be removed in the next
      release of tornado. Please, use the ``tornado_connection_channel``
      instead.*
   -  **tornado_event_socket_port**: The port where outgoing events will
      be written. This should be the port where the Tornado Engine
      listens for incoming events. This entry is mandatory if
      ``tornado_connection_channel`` is set to ``TCP``. If present, this
      value overrides what specified by the
      ``tornado_connection_channel`` entry. *This entry is deprecated
      and will be removed in the next release of tornado. Please, use
      the ``tornado_connection_channel`` instead.*
   -  **message_queue_size**: The in-memory buffer size for Events. It
      makes the application resilient to Tornado Engine crashes or
      temporary unavailability. When Tornado restarts, all messages in
      the buffer will be sent. When the buffer is full, the collector
      will start discarding older messages first.
   -  **uds_path**: The Unix Socket path on which the collector will
      listen for incoming emails.
   -  **tornado_connection_channel**: The channel to send events to
      Tornado. It contains the set of entries required to configure a
      *Nats* or a *TCP* connection. *Beware that this entry will be
      taken into account only if ``tornado_event_socket_ip`` and
      ``tornado_event_socket_port`` are not provided.*

      -  In case of connection using *Nats*, these entries are
         mandatory:

         -  **nats.client.addresses**: The addresses of the NATS server.
         -  **nats.client.auth.type**: The type of authentication used
            to authenticate to NATS (Optional. Valid values are ``None``
            and ``Tls``. Defaults to ``None`` if not provided).
         -  **nats.client.auth.path_to_pkcs12_bundle**: The path to a
            PKCS12 file that will be used for authenticating to NATS
            (Mandatory if ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.pkcs12_bundle_password**: The password to
            decrypt the provided PKCS12 file (Mandatory if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.path_to_root_certificate**: The path to a
            root certificate (in ``.pem`` format) to trust in addition
            to system's trust root. May be useful if the NATS server is
            not trusted by the system as default. (Optional, valid if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.subject**: The NATS Subject where tornado will
            subscribe and listen for incoming events.

      -  In case of connection using *TCP*, these entries are mandatory:

         -  **tcp_socket_ip**: The IP address where outgoing events will
            be written. This should be the address where the Tornado
            Engine listens for incoming events.
         -  **tcp_socket_port**: The port where outgoing events will be
            written. This should be the port where the Tornado Engine
            listens for incoming events.

More information about the logger configuration is available in the
:ref:`tornado-common-logger` documentation.

The default **config-dir** value can be customized at build time by
specifying the environment variable
*TORNADO_EMAIL_COLLECTOR_CONFIG_DIR_DEFAULT*. For example, this will
build an executable that uses */my/custom/path* as the default value:

.. code:: bash

   TORNADO_EMAIL_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo
   build

An example of a full startup command is:

.. code:: bash

   ./tornado_email_collector \
         --config-dir=/tornado-email-collector/config \

In this example the Email Collector starts up and then reads the
configuration from the */tornado-email-collector/config* directory.

.. _tornado-rsyslog-collector-conf:

Tornado Rsyslog Collector Configuration
+++++++++++++++++++++++++++++++++++++++

The executable configuration is based partially on configuration files,
and partially on command line parameters.

The available startup parameters are:

-  **config-dir**: The filesystem folder from which the collector
   configuration is read. The default path is
   */etc/tornado_rsyslog_collector/*.

In addition to these parameters, the following configuration entries are
available in the file *'config-dir'/rsyslog_collector.toml*:

-  **logger**:

   -  **level**: The Logger level; valid values are *trace*, *debug*,
      *info*, *warn*, and *error*.
   -  **stdout**: Determines whether the Logger should print to standard
      output. Valid values are ``true`` and ``false``.
   -  **file_output_path**: A file path in the file system; if provided,
      the Logger will append any output to it.

-  **rsyslog_collector**:

   -  **tornado_event_socket_ip**: The IP address where outgoing events
      will be written. This should be the address where the Tornado
      Engine listens for incoming events. If present, this value
      overrides what specified by the ``tornado_connection_channel``
      entry. *This entry is deprecated and will be removed in the next
      release of tornado. Please, use the ``tornado_connection_channel``
      instead.*
   -  **tornado_event_socket_port**: The port where outgoing events will
      be written. This should be the port where the Tornado Engine
      listens for incoming events. This entry is mandatory if
      ``tornado_connection_channel`` is set to ``TCP``. If present, this
      value overrides what specified by the
      ``tornado_connection_channel`` entry. *This entry is deprecated
      and will be removed in the next release of tornado. Please, use
      the ``tornado_connection_channel`` instead.*
   -  **message_queue_size**: The in-memory buffer size for Events. It
      makes the application resilient to Tornado Engine crashes or
      temporary unavailability. When Tornado restarts, all messages in
      the buffer will be sent. When the buffer is full, the collector
      will start discarding older messages first.
   -  **tornado_connection_channel**: The channel to send events to
      Tornado. It contains the set of entries required to configure a
      *Nats* or a *TCP* connection. *Beware that this entry will be
      taken into account only if ``tornado_event_socket_ip`` and
      ``tornado_event_socket_port`` are not provided.*

      -  In case of connection using *Nats*, these entries are
         mandatory:

         -  **nats.client.addresses**: The addresses of the NATS server.
         -  **nats.client.auth.type**: The type of authentication used
            to authenticate to NATS (Optional. Valid values are ``None``
            and ``Tls``. Defaults to ``None`` if not provided).
         -  **nats.client.auth.path_to_pkcs12_bundle**: The path to a
            PKCS12 file that will be used for authenticating to NATS
            (Mandatory if ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.pkcs12_bundle_password**: The password to
            decrypt the provided PKCS12 file (Mandatory if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.path_to_root_certificate**: The path to a
            root certificate (in ``.pem`` format) to trust in addition
            to system's trust root. May be useful if the NATS server is
            not trusted by the system as default. (Optional, valid if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.subject**: The NATS Subject where tornado will
            subscribe and listen for incoming events.

      -  In case of connection using *TCP*, these entries are mandatory:

         -  **tcp_socket_ip**: The IP address where outgoing events will
            be written. This should be the address where the Tornado
            Engine listens for incoming events.
         -  **tcp_socket_port**: The port where outgoing events will be
            written. This should be the port where the Tornado Engine
            listens for incoming events.

More information about the logger configuration is available in the 
:ref:`tornado-common-logger` documentation.

The default **config-dir** value can be customized at build time by
specifying the environment variable
*TORNADO_RSYSLOG_COLLECTOR_CONFIG_DIR_DEFAULT*. For example, this will
build an executable that uses */my/custom/path* as the default value:

.. code:: bash

   TORNADO_RSYSLOG_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build

.. _tornado-webhook-collector-conf:

Tornado Webhook Collector Configuration
+++++++++++++++++++++++++++++++++++++++

The executable configuration is based partially on configuration files,
and partially on command line parameters.

The available startup parameters are:

-  **config-dir**: The filesystem folder from which the collector
   configuration is read. The default path is
   */etc/tornado_webhook_collector/*.
-  **webhooks-dir**: The folder where the Webhook configurations are
   saved in JSON format; this folder is relative to the ``config_dir``.
   The default value is */webhooks/*.

In addition to these parameters, the following configuration entries are
available in the file *'config-dir'/webhook_collector.toml*:

-  **logger**:

   -  **level**: The Logger level; valid values are *trace*, *debug*,
      *info*, *warn*, and *error*.
   -  **stdout**: Determines whether the Logger should print to standard
      output. Valid values are ``true`` and ``false``.
   -  **file_output_path**: A file path in the file system; if provided,
      the Logger will append any output to it.

-  **webhook_collector**:

   -  **tornado_event_socket_ip**: The IP address where outgoing events
      will be written. This should be the address where the Tornado
      Engine listens for incoming events. If present, this value
      overrides what specified by the ``tornado_connection_channel``
      entry. *This entry is deprecated and will be removed in the next
      release of tornado. Please, use the ``tornado_connection_channel``
      instead.*
   -  **tornado_event_socket_port**: The port where outgoing events will
      be written. This should be the port where the Tornado Engine
      listens for incoming events. This entry is mandatory if
      ``tornado_connection_channel`` is set to ``TCP``. If present, this
      value overrides what specified by the
      ``tornado_connection_channel`` entry. *This entry is deprecated
      and will be removed in the next release of tornado. Please, use
      the ``tornado_connection_channel`` instead.*
   -  **message_queue_size**: The in-memory buffer size for Events. It
      makes the application resilient to errors or temporary
      unavailability of the Tornado connection channel. When the
      connection on the channel is restored, all messages in the buffer
      will be sent. When the buffer is full, the collector will start
      discarding older messages first.
   -  **server_bind_address**: The IP to bind the HTTP server to.
   -  **server_port**: The port to be used by the HTTP Server.
   -  **tornado_connection_channel**: The channel to send events to
      Tornado. It contains the set of entries required to configure a
      *Nats* or a *TCP* connection. *Beware that this entry will be
      taken into account only if ``tornado_event_socket_ip`` and
      ``tornado_event_socket_port`` are not provided.*

      -  In case of connection using *Nats*, these entries are
         mandatory:

         -  **nats.client.addresses**: The addresses of the NATS server.
         -  **nats.client.auth.type**: The type of authentication used
            to authenticate to NATS (Optional. Valid values are ``None``
            and ``Tls``. Defaults to ``None`` if not provided).
         -  **nats.client.auth.path_to_pkcs12_bundle**: The path to a
            PKCS12 file that will be used for authenticating to NATS
            (Mandatory if ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.pkcs12_bundle_password**: The password to
            decrypt the provided PKCS12 file (Mandatory if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.path_to_root_certificate**: The path to a
            root certificate (in ``.pem`` format) to trust in addition
            to system's trust root. May be useful if the NATS server is
            not trusted by the system as default. (Optional, valid if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.subject**: The NATS Subject where tornado will
            subscribe and listen for incoming events.

      -  In case of connection using *TCP*, these entries are mandatory:

         -  **tcp_socket_ip**: The IP address where outgoing events will
            be written. This should be the address where the Tornado
            Engine listens for incoming events.
         -  **tcp_socket_port**: The port where outgoing events will be
            written. This should be the port where the Tornado Engine
            listens for incoming events.

More information about the logger configuration can be found in the
:ref:`tornado-common-logger` documentation.

The default **config-dir** value can be customized at build time by
specifying the environment variable
*TORNADO_WEBHOOK_COLLECTOR_CONFIG_DIR_DEFAULT*. For example, this will
build an executable that uses */my/custom/path* as the default value:

.. code:: bash

   TORNADO_WEBHOOK_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 

An example of a full startup command is:

.. code:: bash

   ./tornado_webhook_collector \
         --config-dir=/tornado-webhook-collector/config

In this example the Webhook Collector starts up and then reads the
configuration from the */tornado-webhook-collector/config* directory.

Webhooks Configuration
++++++++++++++++++++++

As described before, the two startup parameters *config-dir* and
*webhooks-dir* determine the path to the Webhook configurations, and
each webhook is configured by providing *id*, *token* and
*collector_config*.

As an example, consider how to configure a webhook for a repository
hosted on `Github <https://github.com/>`__.

If we start the application using the command line provided in the
previous section, the webhook configuration files should be located in
the */tornado-webhook-collector/config/webhooks* directory. Each
configuration is saved in a separate file in that directory in JSON
format (the order shown in the directory is not necessarily the order in
which the hooks are processed)::

   /tornado-webhook-collector/config/webhooks
                    |- github.json
                    |- bitbucket_first_repository.json
                    |- bitbucket_second_repository.json
                    |- ...

An example of valid content for a Webhook configuration JSON file is:

.. code:: json

   {
     "id": "github_repository",
     "token": "secret_token",
     "collector_config": {
       "event_type": "${commits[0].committer.name}",
       "payload": {
         "source": "github",
         "ref": "${ref}",
         "repository_name": "${repository.name}"
       }
     }
   }

This configuration assumes that this endpoint has been created:

**http(s)://collector_ip:collector_port/event/github_repository**

However, the Github webhook issuer must pass the token at each call.
Consequently, the actual URL to be called will have this structure:

**http(s)://collector_ip:collector_port/event/github_repository?token=secret_token**

**Security warning:** Since the security token is present in the query
string, it is extremely important that the webhook collector is always
deployed with HTTPS in production. Otherwise, the token will be sent
unencrypted along with the entire URL.

Consequently, if the public IP of the collector is, for example,
35.35.35.35 and the server port is 1234, in Github, the webhook settings
page should look like in :numref:`figure-github-webhook`.

.. _figure-github-webhook:

.. figure:: /img/github_webhook_01.png

   Configuring a GitHub webhook.

Finally, the *collector_config* configuration entry determines the
content of the tornado Event associated with each webhook input.

So for example, if Github sends this JSON (only the relevant parts shown
here):

.. code:: json

   {
     "ref": "refs/heads/master",
     "commits": [
       {
         "id": "33ad3a6df86748011ee8d5cef13d206322abc68e",
         "committer": {
           "name": "GitHub",
           "email": "noreply@github.com",
           "username": "web-flow"
         }
       }
     ],
     "repository": {
       "id": 123456789,
       "name": "webhook-test"
     }
   }

then the resulting Event will be:

.. code:: json

   {
     "type": "GitHub",
     "created_ms": 1554130814854,
     "payload": {
       "source": "github",
       "ref": "refs/heads/master",
       "repository_name": "webhook-test"
     }
   }

The Event creation logic is handled internally by the JMESPath
collector, a detailed description of which is available in its
:ref:`specific documentation <tornado-jmespath-collector>`.

.. _tornado-nats-json-collector-conf:

Tornado Nats JSON Collector Configuration
+++++++++++++++++++++++++++++++++++++++++

The executable configuration is based partially on configuration files,
and partially on command line parameters.

The available startup parameters are:

-  **config-dir**: The filesystem folder from which the collector
   configuration is read. The default path is
   */etc/tornado_nats_json_collector/*.
-  **topics-dir**: The folder where the topic configurations are saved
   in JSON format; this folder is relative to the ``config_dir``. The
   default value is */topics/*.

In addition to these parameters, the following configuration entries are
available in the file *'config-dir'/nats_json_collector.toml*:

-  **logger**:

   -  **level**: The Logger level; valid values are *trace*, *debug*,
      *info*, *warn*, and *error*.
   -  **stdout**: Determines whether the Logger should print to standard
      output. Valid values are ``true`` and ``false``.
   -  **file_output_path**: A file path in the file system; if provided,
      the Logger will append any output to it.

-  **nats_json_collector**:

   -  **message_queue_size**: The in-memory buffer size for Events. It
      makes the application resilient to errors or temporary
      unavailability of the Tornado connection channel. When the
      connection on the channel is restored, all messages in the buffer
      will be sent. When the buffer is full, the collector will start
      discarding older messages first.
   -  **nats_client.addresses**: The addresses of the NATS server.
   -  **nats_client.auth.type**: The type of authentication used to
      authenticate to NATS (Optional. Valid values are ``None`` and
      ``Tls``. Defaults to ``None`` if not provided).
   -  **nats_client.auth.path_to_pkcs12_bundle**: The path to a PKCS12
      file that will be used for authenticating to NATS (Mandatory if
      ``nats_client.auth.type`` is set to ``Tls``).
   -  **nats_client.auth.pkcs12_bundle_password**: The password to
      decrypt the provided PKCS12 file (Mandatory if
      ``nats_client.auth.type`` is set to ``Tls``).
   -  **nats_client.auth.path_to_root_certificate**: The path to a root
      certificate (in ``.pem`` format) to trust in addition to system's
      trust root. May be useful if the NATS server is not trusted by the
      system as default. (Optional, valid if ``nats_client.auth.type``
      is set to ``Tls``).
   -  **tornado_connection_channel**: The channel to send events to
      Tornado. It contains the set of entries required to configure a
      *Nats* or a *TCP* connection.

      -  In case of connection using *Nats*, these entries are
         mandatory:

         -  **nats_subject**: The NATS Subject where tornado will
            subscribe and listen for incoming events.

      -  In case of connection using *TCP*, these entries are mandatory:

         -  **tcp_socket_ip**: The IP address where outgoing events will
            be written. This should be the address where the Tornado
            Engine listens for incoming events.
         -  **tcp_socket_port**: The port where outgoing events will be
            written. This should be the port where the Tornado Engine
            listens for incoming events.

More information about the logger configurationis available in the 
:ref:`tornado-common-logger` documentation.

The default **config-dir** value can be customized at build time by
specifying the environment variable
*TORNADO_NATS_JSON_COLLECTOR_CONFIG_DIR_DEFAULT*. For example, this will
build an executable that uses */my/custom/path* as the default value:

.. code:: bash

   TORNADO_NATS_JSON_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 

An example of a full startup command is:

.. code:: bash

   ./tornado_nats_json_collector \
         --config-dir=/tornado-nats-json-collector/config

In this example the Nats JSON Collector starts up and then reads the
configuration from the */tornado-nats-json-collector/config* directory.

Topics Configuration
++++++++++++++++++++

As described before, the two startup parameters *config-dir* and
*topics-dir* determine the path to the topic configurations, and each
topic is configured by providing *nats_topics* and *collector_config*.

As an example, consider how to configure a "simple_test" topic.

If we start the application using the command line provided in the
previous section, the topics configuration files should be located in
the */tornado-nats-json-collector/config/topics* directory. Each
configuration is saved in a separate file in that directory in JSON
format (the order shown in the directory is not necessarily the order in
which the topics are processed)::

   /tornado-nats-json-collector/config/topics
                    |- simple_test.json
                    |- something_else.json
                    |- ...

An example of valid content for a Topic configuration JSON file is:

.. code:: json

   {
     "nats_topics": ["simple_test_one", "simple_test_two"],
     "collector_config": {
       "event_type": "${content.type}",
       "payload": {
         "ref": "${content.ref}",
         "repository_name": "${repository}"
       }
     }
   }

With this configuration, two subscriptions are created to the Nats
topics *simple_test_one* and *simple_test_two*. Messages received by
those topics are processed using the *collector_config* that determines
the content of the tornado Event associated with them.

It is important to note that, if a Nats topic name is used more than
once, then the collector will perfom multiple subscriptions accordingly.
This can happen if a topic name is duplicated into the *nats_topics*
array or in multiple JSON files.

So for example, if this JSON message is received:

.. code:: json

   {
     "content": {
       "type": "content_type",
       "ref": "refs/heads/master"
     },
     "repository": {
       "id": 123456789,
       "name": "webhook-test"
     }
   }

then the resulting Event will be:

.. code:: json

   {
     "type": "content_type",
     "created_ms": 1554130814854,
     "payload": {
       "ref": "refs/heads/master",
       "repository": {
           "id": 123456789,
           "name": "webhook-test"
         }
     }
   }

The Event creation logic is handled internally by the JMESPath
collector, a detailed description of which is available in its
:ref:`specific documentation <tornado-jmespath-collector>`.

.. rubric:: Default values

The *collector_config* section and all of its internal entries are
optional. If not provided explicitly, the collector will use these
predefined values:

-  When the *collector_config.event_type* is not provided, the name of
   the Nats topic that sent the message is used as Event type.
-  When the *collector_config.payload* is not provided, the entire
   source message is included in the payload of the generated Event with
   the key *data*.

Consequently, the simplest valid topic configuration contains only the
*nats_topics*:

.. code:: json

   {
     "nats_topics": ["subject_one", "subject_two"]
   }

The above one is equivalent to:

.. code:: json

   {
     "nats_topics": ["subject_one", "subject_two"],
     "collector_config": {
       "payload": {
         "data": "${@}"
       }
     }
   }

In this case the generated Tornado Events have *type* equals to the
topic name and the whole source data in their payload.

.. _tornado-icinga-collector-conf:

Tornado Icinga2 Collector Configuration
+++++++++++++++++++++++++++++++++++++++

The executable configuration is based partially on configuration files,
and partially on command line parameters.

The available startup parameters are:

-  **config-dir**: The filesystem folder from which the collector
   configuration is read. The default path is
   */etc/tornado_icinga2_collector/*.
-  **streams_dir**: The folder where the Stream configurations are saved
   in JSON format; this folder is relative to the ``config_dir``. The
   default value is */streams/*.

In addition to these parameters, the following configuration entries are
available in the file *'config-dir'/icinga2_collector.toml*:

-  **logger**:

   -  **level**: The Logger level; valid values are *trace*, *debug*,
      *info*, *warn*, and *error*.
   -  **stdout**: Determines whether the Logger should print to standard
      output. Valid values are ``true`` and ``false``.
   -  **file_output_path**: A file path in the file system; if provided,
      the Logger will append any output to it.

-  **icinga2_collector**

   -  **tornado_event_socket_ip**: The IP address where outgoing events
      will be written. This should be the address where the Tornado
      Engine listens for incoming events. If present, this value
      overrides what specified by the ``tornado_connection_channel``
      entry. *This entry is deprecated and will be removed in the next
      release of tornado. Please, use the ``tornado_connection_channel``
      instead.*
   -  **tornado_event_socket_port**: The port where outgoing events will
      be written. This should be the port where the Tornado Engine
      listens for incoming events. This entry is mandatory if
      ``tornado_connection_channel`` is set to ``TCP``. If present, this
      value overrides what specified by the
      ``tornado_connection_channel`` entry. *This entry is deprecated
      and will be removed in the next release of tornado. Please, use
      the ``tornado_connection_channel`` instead.*
   -  **message_queue_size**: The in-memory buffer size for Events. It
      makes the application resilient to Tornado Engine crashes or
      temporary unavailability. When Tornado restarts, all messages in
      the buffer will be sent. When the buffer is full, the collector
      will start discarding older messages first.
   -  **connection**

      -  **server_api_url**: The complete URL of the Icinga2 Event
         Stream API.
      -  **username**: The username used to connect to the Icinga2 APIs.
      -  **password**: The password used to connect to the Icinga2 APIs.
      -  **disable_ssl_verification**: A boolean value. If true, the
         client will not verify the Icinga2 SSL certificate.
      -  **sleep_ms_between_connection_attempts**: In case of connection
         failure, the number of milliseconds to wait before a new
         connection attempt.

   -  **tornado_connection_channel**: The channel to send events to
      Tornado. It contains the set of entries required to configure a
      *Nats* or a *TCP* connection. *Beware that this entry will be
      taken into account only if ``tornado_event_socket_ip`` and
      ``tornado_event_socket_port`` are not provided.*

      -  In case of connection using *Nats*, these entries are
         mandatory:

         -  **nats.client.addresses**: The addresses of the NATS server.
         -  **nats.client.auth.type**: The type of authentication used
            to authenticate to NATS (Optional. Valid values are ``None``
            and ``Tls``. Defaults to ``None`` if not provided).
         -  **nats.client.auth.path_to_pkcs12_bundle**: The path to a
            PKCS12 file that will be used for authenticating to NATS
            (Mandatory if ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.pkcs12_bundle_password**: The password to
            decrypt the provided PKCS12 file (Mandatory if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.client.auth.path_to_root_certificate**: The path to a
            root certificate (in ``.pem`` format) to trust in addition
            to system's trust root. May be useful if the NATS server is
            not trusted by the system as default. (Optional, valid if
            ``nats.client.auth.type`` is set to ``Tls``).
         -  **nats.subject**: The NATS Subject where tornado will
            subscribe and listen for incoming events.

      -  In case of connection using *TCP*, these entries are mandatory:

         -  **tcp_socket_ip**: The IP address where outgoing events will
            be written. This should be the address where the Tornado
            Engine listens for incoming events.
         -  **tcp_socket_port**: The port where outgoing events will be
            written. This should be the port where the Tornado Engine
            listens for incoming events.

More information about the logger configuration is available in the
:ref:`tornado-common-logger` documentation.

The default **config-dir** value can be customized at build time by
specifying the environment variable
*TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT*. For example, this will
build an executable that uses */my/custom/path* as the default value:

.. code:: bash

   TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo
   build

An example of a full startup command is:

.. code:: bash

   ./tornado_webhook_collector \
         --config-dir=/tornado-icinga2-collector/config

In this example the Icinga2 Collector starts up and then reads the
configuration from the */tornado-icinga2-collector/config* directory.

Streams Configuration
+++++++++++++++++++++

As described before, the two startup parameters *config-dir* and
*streams-dir* determine the path to the stream configurations.

For example, if we start the application using the command line provided
in the previous section, the stream configuration files should be
located in the */tornado-icinga2-collector/config/streams* directory.
Each configuration is saved in a separate file in that directory in JSON
format::

   /tornado-icinga2-collector/config/streams
                    |- 001_CheckResults.json
                    |- 002_Notifications.json
                    |- ...

The alphabetical ordering of the files has no impaact on the collector's
logic.

An example of valid content for a stream configuration JSON file is:

.. code:: json

   {
     "stream": {
       "types": ["CheckResult"],
       "queue": "icinga2_CheckResult",
       "filter": "event.check_result.exit_status==2"
     },
     "collector_config": {
       "event_type": "icinga2_event",
       "payload": {
         "source": "icinga2",
         "icinga2_event": "${@}"
        }
     }
   }

This stream subscription will receive all Icinga2 Events of type
'CheckResult' with 'exit_status'=2. It will then produce a Tornado Event
with type 'icinga2_event' and the entire Icinga2 Event in the payload
with key 'icinga2_event'.

The Event creation logic is handled internally by the JMESPath
collector, a detailed description of which is available in its
:ref:`specific documentation <tornado-jmespath-collector>`.

.. _tornado-snmptrap-tcp-collector-conf:

SNMPTrapd TCP Collector Configuration
+++++++++++++++++++++++++++++++++++++

.. rubric:: Prerequisites

This collector has the following runtime requirements:

-  Perl 5.16 or greater
-  Perl packages required:

   -  Cpanel::JSON::XS
   -  NetSNMP::TrapReceiver

You can verify that the Perl packages are available with the command:

.. code:: bash

   $ perl -e 'use Cpanel::JSON::XS;' && \
     perl -e 'use NetSNMP::TrapReceiver;'

If no messages are displayed in the console, then everything is okay;
otherwise, you will see error messages.

In case of missing dependencies, use your system's package manager to
install them.

For example, the required Perl packages can be installed on an Ubuntu
system with:

.. code:: bash

   $ sudo apt install libcpanel-json-xs-perl libsnmp-perl

.. rubric:: Activation

This Collector is meant to be integrated with snmptrapd. To activate it,
put the following line in your *snmptrapd.conf* file::

   perl do "/path_to_the_script/snmptrapd_tcp_collector.pl"; 

Consequently, it is never started manually, but instead will be started,
and managed, directly by *snmptrapd* itself.

At startup, if the collector is configured properly, you should see this
entry either in the logs or in the daemon's standard error output::

   The TCP based snmptrapd_collector was loaded successfully.

.. rubric:: Configuration options

The address of the Tornado Engine TCP instance to which the events are
forwarded is configured with the following environment variables:

-  **TORNADO_ADDR**: the IP address of Tornado Engine. If not specified,
   it will use the default value *127.0.0.1*
-  **TORNADO_PORT**: the port of the TCP socket of Tornado Engine. If
   not specified, it will use the default value *4747*

.. _tornado-snmptrap-nats-collector-conf:

SNMPTrapd NATS Collector Configuration
++++++++++++++++++++++++++++++++++++++

.. rubric::  Prerequisites

This collector has the following runtime requirements:

-  Perl 5.16 or greater
-  Perl packages required:

   -  Cpanel::JSON::XS
   -  Net::NATS::Client
   -  NetSNMP::TrapReceiver

You can verify that the Perl packages are available with the command:

.. code:: bash

   $ perl -e 'use Cpanel::JSON::XS;' && \
     perl -e 'use Net::NATS::Client;' && \
     perl -e 'use NetSNMP::TrapReceiver;'

If no messages are displayed in the console, then everything is okay;
otherwise, you will see error messages.

In case of missing dependencies, use your system's package manager to
install them.

Instructions for installing ``Net::NATS::Client`` are available at its
`official repository <https://github.com/carwynmoore/perl-nats>`__

.. rubric:: Activation

This Collector is meant to be integrated with snmptrapd. To activate it,
put the following line in your *snmptrapd.conf* file::

   perl do "/path_to_the_script/snmptrapd_collector.pl"; 

Consequently, it is never started manually, but instead will be started,
and managed, directly by *snmptrapd* itself.

At startup, if the collector is configured properly, you should see this
entry either in the logs or in the daemon's standard error output:

::

   The snmptrapd_collector for NATS was loaded successfully.

.. rubric:: Configuration options

The information to connect to the NATS Server are provided by the
following environment variables:

-  **TORNADO_NATS_ADDR**: the address of the NATS server. If not
   specified, it will use the default value *127.0.0.1:4222*
-  **TORNADO_NATS_SUBJECT**: the NATS subject where the events are
   published. If not specified, it will use the default value
   *tornado.events*
-  **TORNADO_NATS_SSL_CERT_PEM_FILE**: The filesystem path of a PEM
   certificate. This entry is optional, when provided, the collector
   will use the certificate to connect to the NATS server
-  **TORNADO_NATS_SSL_CERT_KEY**: The filesystem path for the KEY of the
   PEM certificate provided by the *TORNADO_NATS_SSL_CERT_PEM_FILE*
   entry. This entry is mandatory if the
   *TORNADO_NATS_SSL_CERT_PEM_FILE* entry is provided


