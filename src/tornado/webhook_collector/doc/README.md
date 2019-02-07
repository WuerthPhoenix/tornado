# Webhook Collector (binary) 

The Webhook Collector is a standalone HTTP server that listens for REST calls from a generic
webhook, generates Tornado Events from the webhook JSON body, and publishes them on the Tornado
UDS socket.



## How It Works

The webhook collector executable is an HTTP server built on
[actix-web](https://github.com/actix/actix-web).

On startup, it creates a dedicated REST endpoint for each configured webhook.  Calls received by
an endpoint are processed by the embedded
[jmespath collector](../../../collector/jmespath/doc/README.md)
that uses them to produce Tornado Events.  In the final step, the Events are forwarded to the
Tornado executable's UDS socket.

For each webhook, you must provide three values in order to successfully create an endpoint:
- _id_:  The webhook identifier.  This will determine the path of the endpoint; it must be
  unique per webhook.
- _token_:  A security token that the webhook issuer has to include in the URL as part of the
  query string (see the example at the bottom of this page for details).  If the token provided
  by the issuer is missing or does not match the one owned by the collector, then the call will
  be rejected and an HTTP 401 code (UNAUTHORIZED) will be returned.
- *collector_config*:  The transformation logic that converts a webhook JSON object into a Tornado
  Event.  It consists of a JMESPath collector configuration as described in its
  [specific documentation](../../../collector/jmespath/doc/README.md).



## Configuration

The executable configuration is based partially on configuration files and partially on command
line parameters.

The available startup parameters are:
- __logger-stdout__:  Determines whether the Logger should print to standard output. 
  Valid values are `true` and `false`, default to `false`.
- __logger-file-path__:  A file path in the file system; if provided, the Logger will 
  append any output to it.
- __logger-level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
  _error_, defaulting to _warn_.
- __config-dir__:  The filesystem folder from which the Tornado configuration is read.
  The default path is _/etc/tornado_webhook_collector/_.
- __webhooks-dir__:  The folder where the Webhook configurations are saved in JSON format; 
  this folder is relative to the `config_dir`.  The default value is _/webhooks/_.
- __uds-path__:  The Unix Socket path where outgoing events will be written. 
  This should be the path where Tornado is listening for incoming events.
  By default it is _/var/run/tornado/tornado.sock_.
- __uds-mailbox-capacity__:  The in-memory buffer size for Events.  It makes the application
  resilient to Tornado crashes or temporary unavailability.  When Tornado restarts, all messages
  in the buffer will be sent.  When the buffer is full, the collector will start discarding old
  messages.  The default buffer value is `10000`.
- __bind-address__:  The IP to bind the HTTP server to.  The default value is `0.0.0.0`. 
- __server-port__:  The port to be used by the HTTP Server.  The default value is `8080`.

More information about the logger configuration
[is available here](../../../common/logger/doc/README.md).

An example of a full startup command is:
```bash
./tornado_webhook_collector \
      --logger-stdout --logger-level=debug \
      --config-dir=/tornado-webhook-collector/config \
      --bind-address=127.0.0.1
      --server-port=1234
      --uds-path=/tmp/tornado
```

In this example the Webhook Collector does the following:
- Logs to standard output at the *debug* level
- Reads the configuration from the _/tornado-webhook-collector/config_ directory
- Searches for webhook configurations in the _/tornado-webhook-collector/config/webhooks_ directory
- Binds the HTTP server to the IP 127.0.0.1
- Starts the HTTP server at port 1234
- Writes outgoing Events to the UDS socket at _/tmp/tornado_



## Webhooks Configuration

As described before, the two startup parameters _config-dir_ and _webhooks-dir_ determine the path
to the Webhook configurations, and each webhook is configured by providing _id_, _token_ and
_collector_config_.

As an example, consider how to configure a webhook for a repository hosted on 
[Github](https://github.com/).

If we start the application using the command line provided in the previous section, the webhook
configuration files should be located in the _/tornado-webhook-collector/config/webhooks_
directory.  Each configuration is saved in a separate file in that directory in JSON format
(the order shown in the directory is not necessarily the order in which the hooks are processed):
```
/tornado-webhook-collector/config/webhooks
                 |- github.json
                 |- bitbucket_first_repository.json
                 |- bitbucket_second_repository.json
                 |- ...
```

An example of valid content for a Webhook configuration JSON file is:
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

This configuration assumes that this endpoint has been created:

__http(s)://collector_ip:collector_port/event/github_repository__

However, the Github webhook issuer must pass the token at each call.  Consequently, the actual URL
to be called will have this structure:
  
__http(s)://collector_ip:collector_port/event/github_repository?token=secret_token__

__Security warning:__  Since the security token is present in the query string, it is extremely
important that the webhook collector is always deployed with HTTPS in production.  Otherwise, the
token will be sent unencrypted along with the entire URL.

Consequently, if the public IP of the collector is, for example, 35.35.35.35 and the server 
port is 1234, in Github, the webhook settings page should look like this:

![github_webhook_settings](./github_webhook_01.png)

Finally, the *collector_config* configuration entry determines the content of the tornado Event 
associated with each webhook input.

So for example, if Github sends this JSON (only the relevant parts shown here):
```json
{
  "ref": "refs/heads/master",
  ...
  "commits": [
    {
      "id": "33ad3a6df86748011ee8d5cef13d206322abc68e",
      ...
      "committer": {
        "name": "GitHub",
        "email": "noreply@github.com",
        "username": "web-flow"
      }
    }
  ],
  ...
  "repository": {
    "id": 123456789,
    "name": "webhook-test",
    ...
  }
}
``` 

then the resulting Event will be:
```json
{
  "type": "GitHub",
  "created_ts": "2018-12-28T21:45:59.324310806+09:00",
  "payload": {
    "source": "github",
    "ref": "refs/heads/master",
    "repository_name": "webhook-test"
  }
}
```

The Event creation logic is handled internally by the JMESPath collector, a
detailed description of which is available in its 
[specific documentation](../../../collector/jmespath/doc/README.md). 
