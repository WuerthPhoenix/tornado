# Tornado SMS Collector (executable)

The SMS Collector is executed by the smsd daemon as the eventhandler.


## How It Works

The SMS collector executable is built on [tokio](https://tokio.rs/).

It receives as commandline arguments a path to the config files, the event type and a path to the sms file. If the event
is `RECEIVED`, the collector will parse the sms file and then will send the corresponding event to tornado via Nats. 
Should Nats not be available at the moment, it will copy the sms file to the specified path in the configuration. 
If the event could be sent, then the sms file will be deleted afterward.
Following an example of how to call the collector:

```bash
./tornado_sms_collector RECEIVED "/path/to/sms/file" -c "/path/to/conf/folder/"
```

## Configuration

The executable configuration is based partially on configuration files, and partially on command
line parameters.

The available startup parameters are:
- __config-dir__:  The filesystem folder from which the collector configuration is read.
  The default path is _/etc/tornado_icinga2_collector/_.

              
In addition to these parameters, the following configuration entries are available in the 
file _'config-dir'/icinga2_collector.toml_:
- __logger__:
    - __level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
      _error_.
    - __stdout__:  Determines whether the Logger should print to standard output.
      Valid values are `true` and `false`.
    - __file_output_path__:  A file path in the file system; if provided, the Logger will
      append any output to it.
- **sms_collector**
    - **failed_sms_folder**: The folder which contains all the sms that could not be sent to tornado.
    - **tornado_connection_channel**: The channel to send events to Tornado. It contains the set of entries
    required to configure a *Nats*.
    *Beware that this entry will be taken into account only if `tornado_event_socket_ip` and `tornado_event_socket_port` are not provided.*  
        - **nats.client.addresses**: The addresses of the  NATS server.
        - **nats.client.auth.type**:  The type of authentication used to authenticate to NATS
        (Optional. Valid values are `None` and `Tls`. Defaults to `None` if not provided).
        - **nats.client.auth.certificate_path**:  The path to the client certificate (in `.pem` format) that will be
        used for authenticating to NATS.
        (Mandatory if `nats.client.auth.type` is set to `Tls`).
        - **nats.client.auth.private_key_path**:  The path to the client certificate private key (in `.pem` format)
        that will be used for authenticating to NATS.
        - **nats.client.auth.path_to_root_certificate**:  The path to a root certificate (in `.pem` format) to trust in
        addition to system's trust root. May be useful if the NATS server is not trusted by the system as default.
        (Optional, valid if `nats.client.auth.type` is set to `Tls`).
        - **nats.subject**: The NATS Subject where tornado will subscribe and listen for incoming events.


More information about the logger configuration
[is available here](../../common/logger/README.md).
