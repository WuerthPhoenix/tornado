# Tornado Email Collector (Executable)

The _Email Collector Executable_ binary is an executable that generates Tornado Events from
[MIME](https://en.wikipedia.org/wiki/MIME) email inputs.


## How It Works

The Email Collector Executable is built on
[actix](https://github.com/actix/actix).

On startup, it creates a [UDS](https://en.wikipedia.org/wiki/Unix_domain_socket) 
socket where it listens for incoming email messages. 
Each email published on the socket is processed by the embedded
[email collector](../../../collector/email/doc/README.md)
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
[email collector](../../../collector/email/doc/README.md).

Note that the Email Collector will support any email client that works with the
MIME format and UDS sockets.


## Configuration
The executable configuration is based partially on configuration files and partially on command
line parameters.

The available startup parameters are:
- __config-dir__:  The filesystem folder from which the collector configuration is read.
  The default path is _/etc/tornado_email_collector/_.
- __tornado-event-socket-ip__:  The IP address where outgoing events will be written.
  This should be the address where the Tornado Engine is listening for incoming events.
  The default is _127.0.0.1_.
- __tornado-event-socket-port__:  The port where outgoing events will be written.
  This should be the port where the Tornado Engine is listening for incoming events.
  The default is _4747_.
- __message-queue-size__:  The in-memory buffer size for Events. It makes the application
  resilient to Tornado Engine crashes or temporary unavailability.
  When Tornado restarts, all messages in the buffer will be sent.
  When the buffer is full, the collector will start discarding older messages first.
  The default buffer size is `10000` messages.
- __uds-path__: The Unix Socket path on which the collector will listen for incoming emails.
    The default is: _/var/run/tornado_email_collector/email.sock_

In addition to these parameters, the following configuration entries are available in the 
_'config-dir'/email_collector.toml_:
- __logger__:
    - __level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
      _error_.
    - __stdout__:  Determines whether the Logger should print to standard output.
      Valid values are `true` and `false`.
    - __file_output_path__:  A file path in the file system; if provided, the Logger will
      append any output to it.
  
More information about the logger configuration
[is available here](../../../common/logger/doc/README.md).


An example of a full startup command is:
```bash
./tornado_email_collector \
      --config-dir=/tornado-email-collector/config \
      --tornado-event-socket-ip=tornado_server_ip \
      --tornado-event-socket-port=4747 \
      --uds-path=/my/custom/socket/path
```

In this example the Email Collector does the following:
- Reads the configuration from the _/tornado-email-collector/config_ directory
- Writes outgoing Events to the TCP socket at _tornado_server_ip:4747_
- Listens for incoming emails on the local Unix Socket _/my/custom/socket/path_  

