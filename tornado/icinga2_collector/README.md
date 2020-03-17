# Tornado Icinga2 Collector (executable)

The Icinga2 Collector subscribes to the 
[Icinga2 API event streams](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-streams),
generates Tornado Events from the 
Icinga2 Events, and publishes them on the Tornado Engine TCP address.


## How It Works

The Icinga2 collector executable is built on
[actix](https://github.com/actix/actix).

On startup, it connects to an existing [Icinga2 Server API](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/) and subscribes to user defined [Event Streams](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-streams). 
Each Icinga2 Event published on the stream, is processed by the embedded
[jmespath collector](../../collector/jmespath/README.md)
that uses them to produce Tornado Events which are, finally, forwarded to the
Tornado Engine's TCP address.

More than one stream subscription can be defined.
For each stream, you must provide two values in order to successfully create a subscription:
- _stream_: the stream configuration composed of:
    - _types_: An array of 
    [Icinga2 Event types](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-stream-types);
    - _queue_: A unique queue name used by Icinga2 to identify the stream;
    - _filter_: An optional Event Stream filter. 
    Additional information about the filter can be found in the [official documentation](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-stream-filter).
- *collector_config*:  The transformation logic that converts an Icinga2 Event into a Tornado
  Event. It consists of a JMESPath collector configuration as described in its
  [specific documentation](../../collector/jmespath/README.md).

__Note__: Based on the [Icinga2 Event Streams documentation](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#event-streams), multiple HTTP clients can use the same queue name as long as they use the same event types and filter.

## Configuration

The executable configuration is based partially on configuration files, and partially on command
line parameters.

The available startup parameters are:
- __config-dir__:  The filesystem folder from which the collector configuration is read.
  The default path is _/etc/tornado_icinga2_collector/_.
- __streams_dir__:  The folder where the Stream configurations are saved in JSON format;
  this folder is relative to the `config_dir`. The default value is _/streams/_.

              
In addition to these parameters, the following configuration entries are available in the 
file _'config-dir'/icinga2_collector.toml_:
- __logger__:
    - __level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
      _error_.
    - __stdout__:  Determines whether the Logger should print to standard output.
      Valid values are `true` and `false`.
    - __file_output_path__:  A file path in the file system; if provided, the Logger will
      append any output to it.
- **icinga2_collector**
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
    - **connection**
        - __server_api_url__: The complete URL of the Icinga2 Event Stream API.
        - __username__: The username used to connect to the Icinga2 APIs.
        - __password__: The password used to connect to the Icinga2 APIs.
        - __disable_ssl_verification__: A boolean value. If true, 
        the client will not verify the Icinga2 SSL certificate.
        - __sleep_ms_between_connection_attempts__: In case of connection failure, the number of
          milliseconds to wait before a new connection attempt.
    - **tornado_connection_channel**: The channel to send events to Tornado. It contains the set of entries
    required to configure a *NatsStreaming* or a *TCP* connection.
    *Beware that this entry will be taken into account only if `tornado_event_socket_ip` and `tornado_event_socket_port` are not provided.*  
        - In case of connection using *NatsStreaming*, these entries are mandatory:
            - **nats_streaming.base.addresses**: The addresses of the  NATS streaming server.
            - **nats_streaming.base.subject**: The NATS streaming Subject where tornado will subscribe and listen for incoming events.
            - **nats_streaming.base.cluster_id**: The NATS streaming cluster id to connect to.
            - **nats_streaming.base.client_id**: The unique client id to connect to NATS streaming.
        - In case of connection using *TCP*, these entries are mandatory:
            - **tcp_socket_ip**:  The IP address where outgoing events will be written.
              This should be the address where the Tornado Engine listens for incoming events.
            - **tcp_socket_port**:  The port where outgoing events will be written.
              This should be the port where the Tornado Engine listens for incoming events.

More information about the logger configuration
[is available here](../../common/logger/README.md).

The default __config-dir__ value can be customized at build time by specifying
the environment variable *TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT*. 
For example, this will build an executable that uses */my/custom/path* 
as the default value:
```bash
TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT=/my/custom/path cargo build 
```

An example of a full startup command is:
```bash
./tornado_webhook_collector \
      --config-dir=/tornado-icinga2-collector/config 
```

In this example the Icinga2 Collector starts up and then reads 
the configuration from the _/tornado-icinga2-collector/config_ directory.


## Streams Configuration

As described before, the two startup parameters _config-dir_ and _streams-dir_ determine the path
to the stream configurations.

For example, if we start the application using the command line provided in the previous section, the stream
configuration files should be located in the _/tornado-icinga2-collector/config/streams_
directory. Each configuration is saved in a separate file in that directory in JSON format:
```
/tornado-icinga2-collector/config/streams
                 |- 001_CheckResults.json
                 |- 002_Notifications.json
                 |- ...
```

The alphabetical ordering of the files has no impaact on the 
collector's logic.

An example of valid content for a stream configuration JSON file is:
```json
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
```

This stream subscription will receive all Icinga2 Events of type 'CheckResult' with 'exit_status'=2.
It will then produce a Tornado Event with type 'icinga2_event' and the entire 
Icinga2 Event in the payload with key 'icinga2_event'.

The Event creation logic is handled internally by the JMESPath collector, a
detailed description of which is available in its
[specific documentation](../../collector/jmespath/README.md).
