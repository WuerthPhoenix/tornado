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

In Tornado executables, the Logger configuration is usually defined with
command line parameters managed by
`structopt <https://github.com/TeXitoi/structopt>`__. In that case, the
default *level* is set to *warn*, *stdout-output* is disabled and the
*file-output-path* is empty.

For example:

.. code:: bash

   ./tornado --level=info --stdout-output --file-output-path=/tornado/log
