# Tornado Nats JSON Collector (executable)

The Nats JSON Collector is a standalone collector that listens for JSON messages from Nats subjects, 
generates Tornado Events, and sends them to the Tornado Engine.



## How It Works

The Nats JSON collector executable is built on [actix](https://github.com/actix/actix).

On startup, it connects to a set of subjects on a Nats server. Calls received by
 are processed by the embedded
[jmespath collector](../../collector/jmespath/README.md)
that uses them to produce Tornado Events. In the final step, the Events are forwarded to the
Tornado Engine through the configured connection type.

For each topic, you must provide three values in order to successfully configure them:
- _topics_:  A list of Nats topics to which the collector will subscribe.
- *collector_config*:  The transformation logic that converts a JSON object received from Nats into a Tornado
  Event. It consists of a JMESPath collector configuration as described in its
  [specific documentation](../../collector/jmespath/README.md).



## Configuration

The executable configuration is based partially on configuration files, and partially on command
line parameters.

The available startup parameters are:
- __config-dir__:  The filesystem folder from which the collector configuration is read.
  The default path is _/etc/tornado_nats_json_collector/_.
- __topics-dir__:  The folder where the topic configurations are saved in JSON format;
  this folder is relative to the `config_dir`. The default value is _/topics/_.

In addition to these parameters, the following configuration entries are available in the 
file _'config-dir'/nats_json_collector.toml_:
- __logger__:
    - __level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
      _error_.
    - __stdout__:  Determines whether the Logger should print to standard output.
      Valid values are `true` and `false`.
    - __file_output_path__:  A file path in the file system; if provided, the Logger will
      append any output to it.
- **nats_json_collector**:
    - **message_queue_size**:  The in-memory buffer size for Events. It makes the application
      resilient to errors or temporary unavailability of the Tornado connection channel.
      When the connection on the channel is restored, all messages in the buffer will be sent.
      When the buffer is full, the collector will start discarding older messages first.
    - **nats_client.addresses**: The addresses of the  NATS server.
    - **nats_client.auth.type**:  The type of authentication used to authenticate to NATS
    (Optional. Valid values are `None` and `Tls`. Defaults to `None` if not provided).
    - **nats_client.auth.path_to_pkcs12_bundle**:  The path to a PKCS12 file that will be used for authenticating to NATS
    (Mandatory if `nats_client.auth.type` is set to `Tls`).
    - **nats_client.auth.pkcs12_bundle_password**:  The password to decrypt the provided PKCS12 file
    (Mandatory if `nats_client.auth.type` is set to `Tls`).
    - **nats_client.auth.path_to_root_certificate**:  The path to a root certificate (in `.pem` format) to trust in
    addition to system's trust root. May be useful if the NATS server is not trusted by the system as default.
    (Optional, valid if `nats_client.auth.type` is set to `Tls`).
    - **tornado_connection_channel**: The channel to send events to Tornado. It contains the set of entries
    required to configure a *Nats* or a *TCP* connection.
        - In case of connection using *Nats*, these entries are mandatory:
            - **nats_subject**: The NATS Subject where tornado will subscribe and listen for incoming events.
        - In case of connection using *TCP*, these entries are mandatory:
            - **tcp_socket_ip**:  The IP address where outgoing events will be written.
              This should be the address where the Tornado Engine listens for incoming events.
            - **tcp_socket_port**:  The port where outgoing events will be written.
              This should be the port where the Tornado Engine listens for incoming events.

   
More information about the logger configuration
[is available here](../../common/logger/README.md).

The default __config-dir__ value can be customized at build time by specifying
the environment variable *TORNADO_NATS_JSON_COLLECTOR_CONFIG_DIR_DEFAULT*. 
For example, this will build an executable that uses */my/custom/path* 
as the default value:
```bash
TORNADO_NATS_JSON_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 
```

An example of a full startup command is:
```bash
./tornado_nats_json_collector \
      --config-dir=/tornado-nats-json-collector/config
```

In this example the Nats JSON Collector starts up and then reads 
the configuration from the _/tornado-nats-json-collector/config_ directory.


## Topics Configuration

As described before, the two startup parameters _config-dir_ and _webhooks-dir_ determine the path
to the Webhook configurations, and each webhook is configured by providing _id_, _token_ and
_collector_config_.

As an example, consider how to configure a webhook for a repository hosted on
[Github](https://github.com/).

If we start the application using the command line provided in the previous section, the webhook
configuration files should be located in the _/tornado-webhook-collector/config/webhooks_
directory. Each configuration is saved in a separate file in that directory in JSON format
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

However, the Github webhook issuer must pass the token at each call. Consequently, the actual URL
to be called will have this structure:

__http(s)://collector_ip:collector_port/event/github_repository?token=secret_token__

__Security warning:__  Since the security token is present in the query string, it is extremely
important that the webhook collector is always deployed with HTTPS in production. Otherwise, the
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
  "created_ms": 1554130814854,
  "payload": {
    "source": "github",
    "ref": "refs/heads/master",
    "repository_name": "webhook-test"
  }
}
```

The Event creation logic is handled internally by the JMESPath collector, a
detailed description of which is available in its
[specific documentation](../../collector/jmespath/README.md).
