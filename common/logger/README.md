# Common Logger

The *tornado_common_logger* crate contains the logger configuration for the Tornado components.

The logger configuration is based on the following entries:
- __level__:  A list of comma separated logger verbosity levels. Valid values for a level are: *trace*, *debug*, *info*, *warn*, and *error*.
  If only one level is provided, this is used as global logger level. Otherwise, a list of per package levels can be used.
  E.g.:
  - `level=info`: the global logger level is set to *info*
  - `level=warn,tornado=debug`: the global logger level is set to *warn*, the tornado package logger level is set to *debug* 
- __stdout-output__:  A boolean value that determines whether the Logger should print to standard output.
  Valid values are *true* and *false*.
- __file-output-path__:  An optional string that defines a file path in the file system. If
  provided, the Logger will append any output to that file.

The configuration subsection `logger.tracing_elastic_apm` allows to configure the connection to Elastic APM for the tracing
functionality. If this section is not provided traces will be sent to the APM Server.
The following entries can be configured:
- __apm_server_url__:  The url of the Elastic APM Server.
- __apm_server_api_credentials.id__:  (Optional) the ID of the Api Key for authenticating to the Elastic APM server.
- __apm_server_api_credentials.key__:  (Optional) the key of the Api Key for authenticating to the Elastic APM server.
                                       If `apm_server_api_credentials.id` and `apm_server_api_credentials.key` are not
                                       provided, they will be read from the file `<config_dir>/apm_server_api_credentials.json`
  
In Tornado executables, the Logger configuration is usually defined with command line parameters
managed by [clap](https://github.com/clap-rs/clap). In that case, the default _level_
is set to _warn_, _stdout-output_ is disabled and the _file-output-path_ is empty.

For example:
```bash
./tornado --level=info --stdout-output --file-output-path=/tornado/log
```
