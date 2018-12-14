# Archive Executor

The archive executor logs incoming events to the file system.

## Requirements
The archive executor can only write to locally mounted file systems.
In addition, it needs read and write permissions on the folders and files specified in its configuration.


## Configuration
The archive executor has the following configuration entries:

- __file_cache_size__: the number of file descriptors to be cached. 
It can improve overall performances avoiding that the files are continuously opened and closed on each writes.  
- __file_cache_ttl_secs__: the Time To Live of a file descriptor before it is evicted from the cache.
- __base_path__: A directory on the file system where all logs are written.
Based on their type, received actions can log into subdirectories of the base_path; 
however, the archive executor blocks every attempt at writing outside of this folder. 
- __default_path__: A default path where to log all actions that do not specify an archive_type in the payload.  
- __paths__: It maps an archive_type with a subpath relative to the base_path. 
The subpaths can have placeholders, specified with the syntax ```${parameter_name}```, that are replaced at runtime by the action payload's values.

For example, let's suppose the following configuration:
```toml
base_path =  "/tmp"
default_path = "/default/out.log"
file_cache_size = 10
file_cache_ttl_secs = 1

[paths]
"type_one" = "/dir_one/file.log"
"type_two" = "/dir_two/${hostname}/file.log"

``` 

and the three incoming actions:

_action_one_:
```json
{
    "id": "archive",
    "payload": {
        "archive_type": "type_one",
        "event": "__the_incoming_event__"
    }
}
```

_action_two_:
```json
{
    "id": "archive",
    "payload": {
        "archive_type": "type_two",
        "hostname": "net-test",
        "event": "__the_incoming_event__"
    }
}
```

_action_three_:
```json
{
    "id": "archive",
    "payload": {
        "event": "__the_incoming_event__"
    }
}
```

then:

  - _action_one_ is archived into __/tmp/dir_one/file.log__
  - _action_two_ is archived into __/tmp/dir_two/net-test/file.log__
  - _action_three_ is archived into __/tmp/default/out.log__


## How it works
The archive executor expects the action to include in the payload:

1. The event: the event to be archived should be included in the payload with the key "event"
1. The archive type (optional): the archive type can be specified in the payload with the key "archive_type". 
When it is not specified, the default archive path is used; 
otherwise, the one in the in the "paths" configuration corresponding to the "archive_type" key.

When the "archive_path" is specified but there is no correspondence on the configured "paths",
or it is not possible to resolve all the path parameters, then the event will not be archived. 
Instead, an error is returned.  

The event from the payload is written into the lof file in json format, one event per line.

