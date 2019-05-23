# Tornado Email Collector (executable)

The _Email Collector Executable_ binary is an executable that generates Tornado Events from
[MIME](https://en.wikipedia.org/wiki/MIME) email inputs.



## How It Works

The Email Collector Executable is built on
[actix](https://github.com/actix/actix).

On startup, it creates a [UDS](https://en.wikipedia.org/wiki/Unix_domain_socket) 
socket where it will listen for incoming emails. 
Each email published on the socket, is processed by the embedded
[email collector](../../../collector/email/doc/README.md)
to produce Tornado Events which are, finally, forwarded to the
Tornado Engine's TCP address.

Each client that needs to write an email on the socket, should close the connection
as soon as it completes its action; in fact, the Email collector will start
processing the email only when it receives the [EOF](https://en.wikipedia.org/wiki/End-of-file)
signal. Only one email per connection is allowed.

This client behavior can be obtained, for example, 
using [procmail](https://en.wikipedia.org/wiki/Procmail) 
with the following configuration:
```
## .procmailrc file 
MAILDIR=$HOME/Mail                 #you'd better make sure it exists
LOGFILE=$MAILDIR/procmail.log      

# This is where we ask procmail to write to our UDS socket. 
SHELL=/bin/sh
:0
| /usr/bin/socat - /var/run/tornado/email.sock 2>&1
```

A precondition for procmail to work is that the mail server in use is properly
configured to notify procmail whenever it receives a new email. 

For additional information about how the incoming email is processed and
the structure of the generated Event, check the specific documentation of the 
embedded 
[email collector](../../../collector/email/doc/README.md) 


## Configuration

The executable configuration is based on the following command
line parameters:
- __logger-stdout__:  Determines whether the Logger should print to standard output.
  Valid values are `true` and `false`, defaults to `false`.
- __logger-file-path__:  A file path in the file system; if provided, the Logger will
  append any output to it.
- __logger-level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
  _error_, defaulting to _warn_.
- __tornado-event-socket-ip__:  The IP address where outgoing events will be written.
  This should be the address where the Tornado Engine is listening for incoming events.
  The default is _127.0.0.1_.
- __tornado-event-socket-port__:  The port where outgoing events will be written.
  This should be the address where the Tornado Engine is listening for incoming events.
  The default is _4747_.
- __message-queue-size__:  The in-memory buffer size for Events. It makes the application
  resilient to Tornado Engine crashes or temporary unavailability.
  When Tornado restarts, all messages in the buffer will be sent.
  When the buffer is full, the collector will start discarding old messages.
  The default buffer value is `10000`.
- __uds-path__: The Unix Socket path where the collector we will listen for incoming emails.
    The default is: _/var/run/tornado/email.sock_
    
More information about the logger configuration
[is available here](../../../common/logger/doc/README.md).


An example of a full startup command is:
```bash
./tornado_email_collector \
      --logger-stdout --logger-level=debug \
      --tornado-event-socket-ip=tornado_server_ip \
      --tornado-event-socket-port=4747 \
      --uds-path=/my/custom/socket/path
```

In this example the Email Collector does the following:
- Logs to standard output at the *debug* level
- Writes outgoing Events to the TCP socket at _tornado_server_ip:4747_
- Listens for incoming emails on the local Unix Socket _/my/custom/socket/path_  

