# Webhook Collector (binary) 

The webhook collector is a standalone HTTP server that listens for REST calls 
from a generic webhook.

It generates Tornado Events for the webhook JSON body and publish them on the 
Tornado UDS socket.

## How it works

The webhook collector executable is an HTTP server built on [actix-web](https://github.com/actix/actix-web).

At startup, it creates a dedicated REST endpoint
for each configured webhook. Calls received by an endpoint are 
processed by the embedded [jmespath collector](../../../collector/jmespath/doc/README.md)
that produces Tornado Events from them; finally, the Events are forwarded to the Tornado 
executable UDS socket.

For each webhook, to successfully create an endpoint, we have to provide three values:
- _id_: the webhook identifier. This will determine the path of the endpoint; it must be 
  unique per webhook.
- _token_: a security token that the webhook issuer have to include in the query string 
of the URL if the webhook collector endpoint. If the token provided by the issuer is missing
or does not match the one owned by the collector, the call will be rejected.   
- _collector_config_: The transformation logic that converts a webhook JSON into a Tornado Event.
  It consists of a JMESPath collector configuration as described in the [specific 
  documentation](../../../collector/jmespath/doc/README.md).
  

## Configuration

The executable configuration is partially on configuration files
and partially on command line parameters.

The available startup parameters are:
- __logger-stdout__: Determines whether the Logger should print to standard output. 
  Valid values: true, false. False by default.
- __logger-file-path__: A file path in the file system; if provided, the Logger will 
  append any output to it.
- __logger-level__: The Logger level; valid values: _trace_, _debug_, _info_, _warn_, _error_.
  The default value is _warn_.
- __config-dir__: The filesystem folder from where the Tornado configuration is read.
  The default path is _/etc/tornado_webhook_collector/_
- __webhooks_dir__: The folder where the Webhook configurations are saved in JSON format; 
  this folder is relative to the `config_dir`. The default value is _/webhooks/_
- __uds-path__: The Unix Socket path where outgoing events will be written. 
  This should be the path where Tornado is listening for incoming events.
  By default it is _/var/run/tornado/tornado.sock_
- __uds_mailbox_capacity__: The Events in-memory buffer size used when not possible
  to write to the Tornado UDS socket. It makes the application resilient to Tornado crashes.
  When Tornado will be available, all messages in the buffer will be sent. When the buffer
  is full, the collector will start loosing messages.
  The default buffer value is 10000.
- __bind_address__: IP to bind the HTTP server to. The default value is "0.0.0.0". 
- __server_port__: The port to be use by the HTTP Server. The default value is 8080.


More information about the logger configuration are available [here](../../../common/logger/doc/README.md).


An example of a full startup command is:
```bash
./tornado_webhook_collector \
      --logger-stdout --logger-level=debug \
      --config-dir=./tornado-webhook-collector/config \
      --bind_address=127.0.0.1
      --server-port=12345
      --uds-path=/tmp/tornado
```

In this case the Webhook Collector:
- logs to standard output at debug level
- reads the configuration from the _./tornado-webhook-collector/config_ directory,
- searches for webhook configurations in the _./tornado-webhook-collector/config/webhooks_ directory,
- binds the HTTP server to the 127.0.0.1 IP,
- starts the HTTP server at port 12345,
- writes outcoming Events to the UDS socket at _/tmp/tornado_.   


## Webhooks configuration
