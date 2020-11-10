# Upgrade to Async

## ToDo
- [ ] Everything should be single-threaded and `mut`
- [ ] Executor
  - [x] Add `async` to Executor trait
  - [ ] Move to async: Archive
  - [ ] Move to async: Director
  - [ ] Move to async: Elasticsearch
  - [ ] Move to async: foreach
  - [ ] Move to async: Icinga2
  - [ ] Move to async: logger
  - [ ] Move to async: monitoring
  - [ ] Move to async: script
  - [ ] Move to async: smart_monitoring_check_result
  - [ ] Archive executor: check whether the tracing_appender::non_blocking can be used to write to file 
- [ ] Matcher 
  - [ ] Replace FS operation with `async` equivalents
- [ ] Network
  - [ ] Should be !Send and !Sync