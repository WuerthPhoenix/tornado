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
      --config-dir=/tornado-webhook-collector/config \
      --bind_address=127.0.0.1
      --server-port=1234
      --uds-path=/tmp/tornado
```

In this case the Webhook Collector:
- logs to standard output at debug level
- reads the configuration from the _/tornado-webhook-collector/config_ directory,
- searches for webhook configurations in the _/tornado-webhook-collector/config/webhooks_ directory,
- binds the HTTP server to the 127.0.0.1 IP,
- starts the HTTP server at port 1234,
- writes outcoming Events to the UDS socket at _/tmp/tornado_.   


## Webhooks configuration

As described before, the two startup parameters _config-dir_ and _webhooks_dir_ 
determine the path to the Webhook configurations. In addition, it was already reported that 
each webhook configuration is achieved providing _id_, _token_ and _collector_config_.

As an example, let's now configure a webhook for a repository hosted on 
[Github](https://github.com/).

If we start the application using the command line provided on the previous chapter,
the webhook configuration files should be in the _/tornado-webhook-collector/config/webhooks_
directory.
Into this directory, each configuration is saved in a separated file in JSON format:
```
/tornado-webhook-collector/config/webhooks
                 |- github.json
                 |- bitbucket_first_repository.json
                 |- bitbucket_second_repository.json
                 |- ...
```

The alphabetical order has no impact on the configuration.

An example of a valid content for a Webhook configuration JSON file is:
```json
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
```

This configuration predisposes the creation of the endpoint:

__http(s)://collector_ip:collector_port/event/github_repository__

However, the Github webhook issuer must pass the token on each call; consequently,
the final URL of the collector will be:
  
__http(s)://collector_ip:collector_port/event/github_repository?token=secret_token__

Due to the fact that the security token is present in the query string, 
it is extremely important that the webhook collector is always deployed 
in https in production; otherwise, the token will be sent unencrypted along with the
entire URL.

Consequently, if the public IP of the collector is, for example, 35.35.35.35 and the server 
port is 1234, in Github, the webhook settings page will look like:
![github_webhook_settings](./github_webhook_01.png)



TO BE REMOVED:
TO BE REMOVED:
TO BE REMOVED:
TO BE REMOVED:
TO BE REMOVED:
- Its unique name is 'emails_with_temperature'. There cannot be two rules with the same name;
- Its priority is 2. The priority defines the execution order of the rules;
  '0' (zero) is the highest priority and denotes the first rule to be evaluated;
- An Event matches this Rule if, as specified by the _WHERE_ clause, it has type "email", and, 
  as requested by the _WITH_ clause, 
  it is possible to extract the "temperature" variable from the "event.payload.body"; 
- If an Event meets the previously stated requirements, the matcher produces an Action 
  with _id_ "Logger" and a _payload_ with the three entries _type_, _subject_ and _temperature_. 

More information about the Rule's properties and configuration can be found in the 
[matching engine documentation](../../../engine/matcher/doc/README.md) 
