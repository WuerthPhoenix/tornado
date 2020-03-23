# Tornado Email Collector (Executable)

The _Email Collector Executable_ binary is an executable that generates Tornado Events from
[MIME](https://en.wikipedia.org/wiki/MIME) email inputs.


## How It Works

The Email Collector Executable is built on
[actix](https://github.com/actix/actix).

On startup, it creates a [UDS](https://en.wikipedia.org/wiki/Unix_domain_socket) 
socket where it listens for incoming email messages. 
Each email published on the socket is processed by the embedded
[email collector](../../collector/email/README.md)
to produce Tornado Events which are, finally, forwarded to the
Tornado Engine's TCP address.

The UDS socket is created with the same user and group as the tornado_email_collector process,
with permissions set to __770__ (read, write and execute for both the user and the group).

Each client that needs to write an email message to the socket should close the connection
as soon as it completes its action. In fact, the Email Collector Executable will not even start
processing that email until it receives an [EOF](https://en.wikipedia.org/wiki/End-of-file)
signal. Only one email per connection is allowed.


### Procmail Example

This client behavior can be obtained, for instance, by using
[procmail](https://en.wikipedia.org/wiki/Procmail) 
with the following configuration:
```
## .procmailrc file
MAILDIR=$HOME/Mail                 # You should make sure this exists
LOGFILE=$MAILDIR/procmail.log

# This is where we ask procmail to write to our UDS socket.
SHELL=/bin/sh
:0
| /usr/bin/socat - /var/run/tornado_email_collector/email.sock 2>&1
```

A precondition for procmail to work is that the mail server in use must be properly
configured to notify procmail whenever it receives new email.

For additional information about how incoming email is processed and
the structure of the generated Event, check the documentation specific to the 
embedded 
[email collector](../../collector/email/README.md).

Note that the Email Collector will support any email client that works with the
MIME format and UDS sockets.


## Configuration
The executable configuration is based partially on configuration files, and partially on command
line parameters.

The available startup parameters are:
- __config-dir__:  The filesystem folder from which the collector configuration is read.
  The default path is _/etc/tornado_email_collector/_.

In addition to these parameters, the following configuration entries are available in the 
file _'config-dir'/email_collector.toml_:
- __logger__:
    - __level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
      _error_.
    - __stdout__:  Determines whether the Logger should print to standard output.
      Valid values are `true` and `false`.
    - __file_output_path__:  A file path in the file system; if provided, the Logger will
      append any output to it.
- **email_collector**:
    - **tornado_event_socket_ip**: The IP address where outgoing events will be written.
      This should be the address where the Tornado Engine listens for incoming events.
      If present, this value overrides what specified by the `tornado_connection_channel` entry.
      *This entry is deprecated and will be removed in the next release of tornado. Please, use the `tornado_connection_channel` instead.*
    - **tornado_event_socket_port**:  The port where outgoing events will be written.
      This should be the port where the Tornado Engine listens for incoming events.
      This entry is mandatory if `tornado_connection_channel` is set to `TCP`.
      If present, this value overrides what specified by the `tornado_connection_channel` entry.
      *This entry is deprecated and will be removed in the next release of tornado. Please, use the `tornado_connection_channel` instead.*
    - **message_queue_size**:  The in-memory buffer size for Events. It makes the application
      resilient to Tornado Engine crashes or temporary unavailability.
      When Tornado restarts, all messages in the buffer will be sent.
      When the buffer is full, the collector will start discarding older messages first.
    - **uds_path**: The Unix Socket path on which the collector will listen for incoming emails.
    - **tornado_connection_channel**: The channel to send events to Tornado. It contains the set of entries
    required to configure a *NatsStreaming* or a *TCP* connection.
    *Beware that this entry will be taken into account only if `tornado_event_socket_ip` and `tornado_event_socket_port` are not provided.*  
        - In case of connection using *NatsStreaming*, these entries are mandatory:
            - **nats.client.addresses**: The addresses of the  NATS streaming server.
            - **nats.client.subject**: The NATS streaming Subject where tornado will subscribe and listen for incoming events.
            - **nats.client.cluster_id**: The NATS streaming cluster id to connect to.
            - **nats.client.client_id**: The unique client id to connect to NATS streaming.
        - In case of connection using *TCP*, these entries are mandatory:
            - **tcp_socket_ip**:  The IP address where outgoing events will be written.
              This should be the address where the Tornado Engine listens for incoming events.
            - **tcp_socket_port**:  The port where outgoing events will be written.
              This should be the port where the Tornado Engine listens for incoming events.

More information about the logger configuration
[is available here](../../common/logger/README.md).

The default __config-dir__ value can be customized at build time by specifying
the environment variable *TORNADO_EMAIL_COLLECTOR_CONFIG_DIR_DEFAULT*. 
For example, this will build an executable that uses */my/custom/path* 
as the default value:
```bash
TORNADO_EMAIL_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 
```

An example of a full startup command is:
```bash
./tornado_email_collector \
      --config-dir=/tornado-email-collector/config \
```

In this example the Email Collector starts up and then reads the configuration from 
the _/tornado-email-collector/config_ directory.


