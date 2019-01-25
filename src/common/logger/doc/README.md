# Common Logger

The *tornado_common_logger* crate contains the logger configuration of the Tornado components.  
The current implementation is based on [fern](https://github.com/daboross/fern).

The configuration is based on three entries:
- __level__: Defines the logger verbosity. Valid values and: trace, debug, info, warn, error;
- __stdout-output__: A boolean value that determines whether the Logger should print to 
  standard output. Valid values: true, false;
- __file-output-path__: An optional String value defining a file path in the file system; 
  if provided, the Logger will append any output to it.
  
In Tornado executables, the logger configuration is usually defined with command line parameters
managed by [structopt](https://github.com/TeXitoi/structopt). In that case, the default _level_ is 
set to _warn_, the _stdout-output_ is disabled and the _file-output-path_ is empty. 

E.g.
```bash
./tornado --level=info --stdout-output --file-output-path=/tornado/log
``` 