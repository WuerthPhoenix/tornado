
Collector Common
````````````````

The *tornado_collector_common* crate contains the Trait definitions for
the Collectors. A *Collector* is an event data source. It receives
information from one or more unstructured sources (e.g. emails or log
files), produces structured Events, and sends them to the Tornado
engine.


.. _tornado-json-collectors:

JSON Collectors
```````````````

These are Collectors that receive an input in JSON and unmarshall it
into an internal Event struct.

There are currently two available implementations:

1. The *JsonEventCollector*
2. The *JsonPayloadCollector*

JsonEventCollector
++++++++++++++++++

The *JsonEventCollector* expects to receive a valid JSON representation
of a Tornado Event as input. It is used internally by Tornado to
unmarshall Events received, for example, from a TCP or UDS socket.

The JSON input format should respect the Event structure, for example:

.. code:: json

   {
     "type": "email",
     "created_ms": 1554130814854,
     "payload":{
       "subject": "Email subject",
       "body": "Email body",
       "other": {
         "some_text": "some text",
         "a_bool": true,
         "a_number": 123456.789,
         "something_else": {}
       }
     }
   }

JsonPayloadCollector
++++++++++++++++++++

The *JsonPayloadCollector* receives any valid JSON object and creates a
Tornado Event whose payload is that input. For example, the following
input:

.. code:: json

   {
     "@timestamp": "2018-11-01T23:59:59+01:00",
     "host": "neteye01",
     "hostgroups": [
       "windows",
       "database",
       "rome"
     ],
     "icinga_customfields": {
       "snmpcommunity": "secret",
       "os": "windows"
     },
     "severity": "DEBUG",
     "facility": "daemon",
     "syslog-tag": "nfcapd[20747]:",
     "source": "nfcapd",
     "message": " Process_v9: Found options flowset: template 259"
   }

will generate this Event:

.. code:: json

   {
     "type": "event_type_from_config",
     "created_ms": 1554130814854,
     "payload": {
       "@timestamp": "2018-11-01T23:59:59+01:00",
       "host": "neteye01",
       "hostgroups": [
         "windows",
         "database",
         "rome"
       ],
       "icinga_customfields": {
         "snmpcommunity": "secret",
         "os": "windows"
       },
       "severity": "DEBUG",
       "facility": "daemon",
       "syslog-tag": "nfcapd[20747]:",
       "source": "nfcapd",
       "message": " Process_v9: Found options flowset: template 259"
     }
   }

The Event "type" property must be specified when the collector is
instantiated.

.. _tornado-jmespath:

JMESPath Collector
``````````````````

This is a Collector that receives an input in JSON format and allows the
creation of Events using the `JMESPath JSON query
language <http://jmespath.org/>`__.

Configuration
+++++++++++++

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

How it Works
++++++++++++

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

Runtime behavior
++++++++++++++++

When the JMESPath expression returns an array or a map, the entire
object will be inserted as-is into the Event.

However, if a JMESPath expression does not return a valid result, then
no Event is created, and an error is produced.

.. _tornado-email-collector:

Email Collector
```````````````

The *Email Collector* receives a `MIME email
message <https://en.wikipedia.org/wiki/MIME>`__ as input, parses it, and
produces a Tornado Event.

How It Works
++++++++++++

When the *Email Collector* receives a valid `MIME email
message <https://en.wikipedia.org/wiki/MIME>`__ as input, it parses it
and produces a Tornado Event with the extracted data.

For example, given the following input::

   Subject: This is a test email
   Content-Type: multipart/alternative; boundary=foobar
   Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)

   --foobar
   Content-Type: text/plain; charset=utf-8
   Content-Transfer-Encoding: quoted-printable

   This is the plaintext version, in utf-8. Proof by Euro: =E2=82=AC
   --foobar
   Content-Type: text/html
   Content-Transfer-Encoding: base64

   PGh0bWw+PGJvZHk+VGhpcyBpcyB0aGUgPGI+SFRNTDwvYj4gdmVyc2lvbiwgaW4g 
   dXMtYXNjaWkuIFByb29mIGJ5IEV1cm86ICZldXJvOzwvYm9keT48L2h0bWw+Cg== 
   --foobar--

it will generate this Event:

.. code:: json

   {
     "type": "email",
     "created_ms": 1554130814854,
     "payload": {
       "date": 1475417182,
       "subject": "This is a test email",
       "to": "",
       "from": "",
       "cc": "",
       "body": "This is the plaintext version, in utf-8. Proof by Euro: €",
       "attachments": []
     }
   }

If there are attachments, then attachments that are text files will be
in plain text, otherwise they will be encoded in base64.

For example, passing this email with attachments:

.. code:: mime

   From: "Francesco" <francesco@example.com>
   Subject: Test for Mail collector - with attachments
   To: "Benjamin" <benjamin@example.com>,
    francesco <francesco@example.com>
   Cc: thomas@example.com, francesco@example.com
   Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)
   MIME-Version: 1.0
   Content-Type: multipart/mixed;
    boundary="------------E5401F4DD68F2F7A872C2A83"
   Content-Language: en-US

   This is a multi-part message in MIME format.
   --------------E5401F4DD68F2F7A872C2A83
   Content-Type: text/html; charset=utf-8
   Content-Transfer-Encoding: 7bit

   <html>Test for Mail collector with attachments</html>

   --------------E5401F4DD68F2F7A872C2A83
   Content-Type: application/pdf;
    name="sample.pdf"
   Content-Transfer-Encoding: base64
   Content-Disposition: attachment;
    filename="sample.pdf"

   JVBERi0xLjMNCiXi48/TDQoNCjEgMCBvYmoNCjw8DQovVHlwZSAvQ2F0YWxvZw0KT0YNCg==

   --------------E5401F4DD68F2F7A872C2A83
   Content-Type: text/plain; charset=UTF-8;
    name="sample.txt"
   Content-Transfer-Encoding: base64
   Content-Disposition: attachment;
    filename="sample.txt"

   dHh0IGZpbGUgY29udGV4dCBmb3IgZW1haWwgY29sbGVjdG9yCjEyMzQ1Njc4OTA5ODc2NTQz
   MjEK
   --------------E5401F4DD68F2F7A872C2A83--

will generate this Event:

.. code:: json

   {
     "type": "email",
     "created_ms": 1554130814854,
     "payload": {
       "date": 1475417182,
       "subject": "Test for Mail collector - with attachments",
       "to": "\"Benjamin\" <benjamin@example.com>, francesco <francesco@example.com>",
       "from": "\"Francesco\" <francesco@example.com>",
       "cc": "thomas@example.com, francesco@example.com",
       "body": "<html>Test for Mail collector with attachments</html>",
       "attachments": [
         {
           "filename": "sample.pdf",
           "mime_type": "application/pdf",
           "encoding": "base64",
           "content": "JVBERi0xLjMNCiXi48/TDQoNCjEgMCBvYmoNCjw8DQovVHlwZSAvQ2F0YWxvZw0KT0YNCg=="
         },
         {
           "filename": "sample.txt",
           "mime_type": "text/plain",
           "encoding": "plaintext",
           "content": "txt file context for email collector\n1234567890987654321\n"
         }
       ]
     }
   }

Within the Tornado Event, the *filename* and *mime_type* properties of
each attachment are the values extracted from the incoming email.

Instead, the *encoding* property refers to how the *content* is encoded
in the Event itself. It can be one of two types:

-  **plaintext**: The content is included in plain text
-  **base64**: The content is encoded in base64

Particular cases
++++++++++++++++

The email collector follows these rules to generate the Tornado Event:

-  If more than one body is present in the email or its subparts, the
   first valid body found is used, while the others will be ignored
-  Content Dispositions different from *Inline* and *Attachment* are
   ignored
-  Content Dispositions of type *Inline* are processed only if the
   content type is *text/\**
-  The email subparts are not scanned recursively, thus only the
   subparts at the root level are evaluated


Tornado Email Collector (Executable)
````````````````````````````````````

The *Email Collector Executable* binary is an executable that generates
Tornado Events from `MIME <https://en.wikipedia.org/wiki/MIME>`__ email
inputs.

How It Works
++++++++++++

The Email Collector Executable is built on
`actix <https://github.com/actix/actix>`__.

On startup, it creates a `UDS
<https://en.wikipedia.org/wiki/Unix_domain_socket>`__ socket where qit
listens for incoming email messages. Each email published on the
socket is processed by the embedded :ref:`tornado-email-collector` to
produce Tornado Events which are, finally, forwarded to the Tornado
Engine's TCP address.

The UDS socket is created with the same user and group as the
tornado_email_collector process, with permissions set to **770** (read,
write and execute for both the user and the group).

Each client that needs to write an email message to the socket should
close the connection as soon as it completes its action. In fact, the
Email Collector Executable will not even start processing that email
until it receives an `EOF <https://en.wikipedia.org/wiki/End-of-file>`__
signal. Only one email per connection is allowed.

.. rubric:: Procmail Example

This client behavior can be obtained, for instance, by using
`procmail <https://en.wikipedia.org/wiki/Procmail>`__ with the following
configuration::

   ## .procmailrc file
   MAILDIR=$HOME/Mail                 # You should make sure this exists
   LOGFILE=$MAILDIR/procmail.log

   # This is where we ask procmail to write to our UDS socket.
   SHELL=/bin/sh
   :0
   | /usr/bin/socat - /var/run/tornado_email_collector/email.sock 2>&1

A precondition for procmail to work is that the mail server in use must
be properly configured to notify procmail whenever it receives new
email.

For additional information about how incoming email is processed and
the structure of the generated Event, check the documentation specific
to the embedded :ref:`tornado-email-collector`.

Note that the Email Collector will support any email client that works
with the MIME format and UDS sockets.

Configuration
+++++++++++++

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

   TORNADO_EMAIL_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 

An example of a full startup command is:

.. code:: bash

   ./tornado_email_collector \
         --config-dir=/tornado-email-collector/config \

In this example the Email Collector starts up and then reads the
configuration from the */tornado-email-collector/config* directory.

Tornado Rsyslog Collector (executable)
``````````````````````````````````````

The rsyslog Collector binary is an executable that generates Tornado
Events from rsyslog inputs.

How It Works
++++++++++++

This Collector is meant to be integrated with rsyslog’s own logging
through the `omprog
module <https://www.rsyslog.com/doc/v8-stable/configuration/modules/omprog.html>`__.
Consequently, it is never started manually, but instead will be started,
and managed, directly by rsyslog itself.

Here is an example rsyslog configuration template that pipes logs to the
rsyslog-collector (the parameters are explained below)::

   module(load="omprog")

   action(type="omprog"
          binary="/path/to/tornado_rsyslog_collector --some-collector-options")

An example of a fully instantiated startup setup is::

   module(load="omprog")

   action(type="omprog"
          binary="/path/to/rsyslog_collector --config-dir=/tornado-rsyslog-collector/config --tornado-event-socket-ip=tornado_server_ip --tornado-event-socket-port=4747")

..   <!-- This part may only be necessary for non-expert users. Hide until later? -->

Note that all parameters for the *binary* option must be on the same
line. You will need to place this configuration in a file in your
rsyslog directory, for instance::

   /etc/rsyslog.d/tornado.conf

In this example the collector will:

-  Reads the configuration from the */tornado-rsyslog-collector/config*
   directory
-  Write outgoing Events to the TCP socket at tornado_server_ip:4747

The Collector will need to be run in parallel with the Tornado engine
before any events will be processed, for example::

   /opt/tornado/bin/tornado --tornado-event-socket-ip=tornado_server_ip

Under this configuration, rsyslog is in charge of starting the collector
when needed and piping the incoming logs to it. As the last stage, the
Tornado Events generated by the collector are forwarded to the Tornado
Engine's TCP socket.

This integration strategy is the best option for supporting high
performance given massive amounts of log data.

Because the collector expects the input to be in JSON format, **rsyslog
should be pre-configured** to properly pipe its inputs in this form.

Configuration Options
+++++++++++++++++++++

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

Tornado Webhook Collector (executable)
``````````````````````````````````````

The Webhook Collector is a standalone HTTP server that listens for REST
calls from a generic webhook, generates Tornado Events from the webhook
JSON body, and sends them to the Tornado Engine.

How It Works
++++++++++++

The webhook collector executable is an HTTP server built on
`actix-web <https://github.com/actix/actix-web>`__.

On startup, it creates a dedicated REST endpoint for each configured
webhook. Calls received by an endpoint are processed by the embedded
:ref:`tornado-jmespath` that uses them to produce Tornado Events. In
the final step, the Events are forwarded to the Tornado Engine through
the configured connection type.

For each webhook, you must provide three values in order to successfully
create an endpoint:

-  *id*: The webhook identifier. This will determine the path of the
   endpoint; it must be unique per webhook.
-  *token*: A security token that the webhook issuer has to include in
   the URL as part of the query string (see the example at the bottom of
   this page for details). If the token provided by the issuer is
   missing or does not match the one owned by the collector, then the
   call will be rejected and an HTTP 401 code (UNAUTHORIZED) will be
   returned.
-  *collector_config*: The transformation logic that converts a webhook
   JSON object into a Tornado Event. It consists of a JMESPath collector
   configuration as described in its :ref:`specific
   documentation <tornado-jmespath>`.

Configuration
+++++++++++++

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
:ref:`specific documentation <tornado-jmespath>`.

Tornado Nats JSON Collector (executable)
````````````````````````````````````````

The Nats JSON Collector is a standalone collector that listens for JSON
messages on Nats topics, generates Tornado Events, and sends them to the
Tornado Engine.

How It Works
++++++++++++

The Nats JSON collector executable is built on
`actix <https://github.com/actix/actix>`__.

On startup, it connects to a set of topics on a Nats server. Calls
received are then processed by the embedded :ref:`jmespath collector
<tornado-jmespath>` that uses them to produce Tornado Events. In the
final step, the Events are forwarded to the Tornado Engine through the
configured connection type.

For each topic, you must provide two values in order to successfully
configure them:

-  *nats_topics*: A list of Nats topics to which the collector will
   subscribe.
-  *collector_config*: (Optional) The transformation logic that
   converts a JSON object received from Nats into a Tornado Event. It
   consists of a JMESPath collector configuration as described in its
   :ref:`specific documentation <tornado-jmespath>`.

Configuration
+++++++++++++

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
:ref:`specific documentation <tornado-jmespath>`.

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


Tornado Icinga2 Collector (executable)
``````````````````````````````````````

The Icinga2 Collector subscribes to the `Icinga2 API event
streams <https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-streams>`__,
generates Tornado Events from the Icinga2 Events, and publishes them on
the Tornado Engine TCP address.

How It Works
++++++++++++

The Icinga2 collector executable is built on
`actix <https://github.com/actix/actix>`__.

On startup, it connects to an existing `Icinga2 Server API
<https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/>`__ and
subscribes to user defined `Event Streams
<https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-streams>`__.
Each Icinga2 Event published on the stream, is processed by the
embedded :ref:`jmespath collector <tornado-jmespath>` that uses them
to produce Tornado Events which are, finally, forwarded to the Tornado
Engine's TCP address.

More than one stream subscription can be defined. For each stream, you
must provide two values in order to successfully create a subscription:

-  *stream*: the stream configuration composed of:

   -  *types*: An array of `Icinga2 Event
      types <https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-stream-types>`__;
   -  *queue*: A unique queue name used by Icinga2 to identify the
      stream;
   -  *filter*: An optional Event Stream filter. Additional information
      about the filter can be found in the `official
      documentation <https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-stream-filter>`__.

-  *collector_config*: The transformation logic that converts an Icinga2
   Event into a Tornado Event. It consists of a JMESPath collector
   configuration as described in its :ref:`specific
   documentation <tornado-jmespath>`.

.. note:: Based on the `Icinga2 Event Streams documentation
   <https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-streams>`__,
   multiple HTTP clients can use the same queue name as long as they
   use the same event types and filter.

Configuration
+++++++++++++

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

More information about the logger configuration  is available in the 
:ref:`tornado-common-logger` documentation.

The default **config-dir** value can be customized at build time by
specifying the environment variable
*TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT*. For example, this will
build an executable that uses */my/custom/path* as the default value:

.. code:: bash

   TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 

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
:ref:`specific documentation <tornado-jmespath>`.

SNMP Trap Daemon Collectors
```````````````````````````

The \_snmptrapd_collector_s of this package are embedded Perl trap
handlers for Net-SNMP's snmptrapd. When registered as a subroutine in
the Net-SNMP snmptrapd process, they receives snmptrap-specific inputs,
transforms them into Tornado Events, and forwards them to the Tornado
Engine.

There are two collector implementations, the first one sends Events
directly to the Tornado TCP socket and the second one forwards them to a
NATS server.

The implementations rely on the Perl NetSNMP::TrapReceiver package. You
can refer to `its
documentation <https://metacpan.org/pod/NetSNMP::TrapReceiver>`__ for
generic configuration examples and usage advice.

SNMPTrapd TCP Collector Configuration
`````````````````````````````````````

Prerequisites
+++++++++++++

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

Activation
++++++++++

This Collector is meant to be integrated with snmptrapd. To activate it,
put the following line in your *snmptrapd.conf* file::

   perl do "/path_to_the_script/snmptrapd_tcp_collector.pl"; 

Consequently, it is never started manually, but instead will be started,
and managed, directly by *snmptrapd* itself.

At startup, if the collector is configured properly, you should see this
entry either in the logs or in the daemon's standard error output::

   The TCP based snmptrapd_collector was loaded successfully.

Configuration options
+++++++++++++++++++++

The address of the Tornado Engine TCP instance to which the events are
forwarded is configured with the following environment variables:

-  **TORNADO_ADDR**: the IP address of Tornado Engine. If not specified,
   it will use the default value *127.0.0.1*
-  **TORNADO_PORT**: the port of the TCP socket of Tornado Engine. If
   not specified, it will use the default value *4747*

SNMPTrapd NATS Collector Configuration
``````````````````````````````````````

.. _prerequisites-1:

Prerequisites
+++++++++++++

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

.. _activation-1:

Activation
++++++++++

This Collector is meant to be integrated with snmptrapd. To activate it,
put the following line in your *snmptrapd.conf* file::

   perl do "/path_to_the_script/snmptrapd_collector.pl"; 

Consequently, it is never started manually, but instead will be started,
and managed, directly by *snmptrapd* itself.

At startup, if the collector is configured properly, you should see this
entry either in the logs or in the daemon's standard error output:

::

   The snmptrapd_collector for NATS was loaded successfully.

.. _configuration-options-1:

Configuration options
+++++++++++++++++++++

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

How They Work
+++++++++++++

The \_snmptrapd_collector_s receive snmptrapd messages, parse them,
generate Tornado Events and, finally, sends them to Tornado using their
specific communication channel.

The received messages are kept in an in-memory non-persistent buffer
that makes the application resilient to crashes or temporary
unavailability of the communication channel. When the connection to the
channel is restored, all messages in the buffer will be sent. When the
buffer is full, the collectors will start discarding old messages. The
buffer max size is set to ``10000`` messages.

Consider a snmptrapd message that contains the following information::

   PDU INFO:
     version                        1
     errorstatus                    0
     community                      public
     receivedfrom                   UDP: [127.0.1.1]:41543->[127.0.2.2]:162
     transactionid                  1
     errorindex                     0
     messageid                      0
     requestid                      414568963
     notificationtype               TRAP
   VARBINDS:
     iso.3.6.1.2.1.1.3.0            type=67 value=Timeticks: (1166403) 3:14:24.03
     iso.3.6.1.6.3.1.1.4.1.0        type=6  value=OID: iso.3.6.1.4.1.8072.2.3.0.1
     iso.3.6.1.4.1.8072.2.3.2.1     type=2  value=INTEGER: 123456

The collector will produce this Tornado Event:

.. code:: json

   {
      "type":"snmptrapd",
      "created_ms":"1553765890000",
      "payload":{
         "protocol":"UDP",
         "src_ip":"127.0.1.1",
         "src_port":"41543",
         "dest_ip":"127.0.2.2",
         "PDUInfo":{
            "version":"1",
            "errorstatus":"0",
            "community":"public",
            "receivedfrom":"UDP: [127.0.1.1]:41543->[127.0.2.2]:162",
            "transactionid":"1",
            "errorindex":"0",
            "messageid":"0",
            "requestid":"414568963",
            "notificationtype":"TRAP"
         },
         "oids":{
            "iso.3.6.1.2.1.1.3.0":"67",
            "iso.3.6.1.6.3.1.1.4.1.0":"6",
            "iso.3.6.1.4.1.8072.2.3.2.1":"2"
         }
      }
   }

The structure of the generated Event is not configurable.

Testing
+++++++

To test the collector, verify that snmptrapd is installed on the machine
and follow the collector configuration instructions above.

As a prerequisite, the Tornado Engine should be up and running on the
same machine (:ref:`See the dedicated Tornado engine documentation
<tornado-engine>`).

In addition the *snmptrap* tool is required to send fake snmptrapd
messages.

On Ubuntu, both the *snmptrap* tool and the *snmptrapd* daemon can be
installed with:

.. code:: bash

   sudo apt install snmp snmptrapd

You can now start snmptrapd (as root) in a terminal:

.. code:: bash

   # snmptrapd -f -Le

And send fake messages with the command:

.. code:: bash

   $ snmptrap -v 2c -c public localhost '' 1.3.6.1.4.1.8072.2.3.0.1 1.3.6.1.4.1.8072.2.3.2.1 i 123456

If everything is configured correctly, you should see a message in the
snmptrapd standard error and an Event of type *'snmptrapd'* received by
the Tornado Engine.

In the event of authorization errors, and **only for testing purposes**,
you can fix them by adding this line to the *snmptrapd.conf* file (in
Ubuntu you can find it in */etc/snmp/snmptrapd.conf*)::

   disableAuthorization yes

Extending MIBs
++++++++++++++

SNMP relies on MIB (Management Information Base) definition files, but
the *net-snmp* toolkit used in NetEye does not come with a complete set
for all network devices. You may thus find it necessary to add new
definitions when configuring Tornado in your environment.

If you have not previously set up *net-snmp* tools, you can enable the
principle command as follows:::

   yum install /usr/bin/snmptranslate

If your device is already in the system, this command will return its
OID, or else an error::

   # snmptranslate -IR -On snmpTrapOID
   .1.3.6.1.6.3.1.1.4.1
   # snmptranslate -IR -On ciscoLS1010ChassisFanLed
   Unknown object identifier: ciscoLS1010ChassisFanLed

If your device is not known, you can download its MIB file (e.g., from
`Cisco <ftp://ftp.cisco.com/pub/mibs/v2/>`__) and place it in the
default NetEye directory::

   /usr/share/snmp/mibs

You will then need to make *net-snmp* aware of the new configuration and
ensure it is reloaded automatically on reboot. More information can be
found at the `official Net-SNMP
website <http://net-snmp.sourceforge.net/wiki/index.php/TUT:Using_and_loading_MIBS>`__.
