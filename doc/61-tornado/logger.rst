.. _tornado-common-logger:

Common Logger
`````````````

The *tornado_common_logger* crate contains the logger configuration for
the Tornado components.

The configuration is based on three entries:

-  **level**: A list of comma separated logger verbosity levels. Valid
   values for a level are: *trace*, *debug*, *info*, *warn*, and
   *error*. If only one level is provided, this is used as global logger
   level. Otherwise, a list of per package levels can be used. E.g.:

   -  ``level=info``: the global logger level is set to *info*
   -  ``level=warn,tornado=debug``: the global logger level is set to
      *warn*, the tornado package logger level is set to *debug*

-  **stdout-output**: A boolean value that determines whether the Logger
   should print to standard output. Valid values are *true* and *false*.
-  **file-output-path**: An optional string that defines a file path in
   the file system. If provided, the Logger will append any output to
   that file.

The configuration subsection `logger.tracing_elastic_apm` allows to
configure the connection to Elastic APM for the tracing
functionality. If this section is not provided traces will be sent to
the APM Server.  The following entries can be configured:

- __apm_output__: Whether the Logger data should be sent to the
  Elastic APM Server. Valid values are *true* and *false*.
- __apm_server_url__: The url of the Elastic APM Server.
- __apm_server_api_credentials.id__: (Optional) the ID of the Api Key
  for authenticating to the Elastic APM server.
- __apm_server_api_credentials.key__: (Optional) the key of the Api
  Key for authenticating to the Elastic APM server.  If
  `apm_server_api_credentials.id` and `apm_server_api_credentials.key`
  are not provided, they will be read from the file
  `<config_dir>/apm_server_api_credentials.json`


In Tornado executables, the Logger configuration is usually defined with
command line parameters managed by
`structopt <https://github.com/TeXitoi/structopt>`__. In that case, the
default *level* is set to *warn*, *stdout-output* is disabled and the
*file-output-path* is empty.

For example:

.. code:: bash

   ./tornado --level=info --stdout-output --file-output-path=/tornado/log
