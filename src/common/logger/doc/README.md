# Common Logger

The *tornado_common_logger* crate contains the logger configuration for the Tornado components.
The current implementation is based on [fern](https://github.com/daboross/fern).

The configuration is based on three entries:
- __level__:  Defines the logger verbosity.  Valid values are: *trace*, *debug*, *info*, *warn*, and *error*.
- __stdout-output__:  A boolean value that determines whether the Logger should print to standard output.
  Valid values are *true* and *false*.
- __file-output-path__:  An optional string that defines a file path in the file system.  If
  provided, the Logger will append any output to that file.

In Tornado executables, the Logger configuration is usually defined with command line parameters
managed by [structopt](https://github.com/TeXitoi/structopt).  In that case, the default _level_
is set to _warn_, _stdout-output_ is disabled and the _file-output-path_ is empty. 

For example:
```bash
./tornado --level=info --stdout-output --file-output-path=/tornado/log
```
