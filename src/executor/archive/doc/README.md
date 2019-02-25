# Archive Executor

The Archive Executor ia an executor that writes the Events from the received Actions to a file.



## Requirements and Limitations

The archive executor can only write to locally mounted file systems.  In addition, it needs read
and write permissions on the folders and files specified in its configuration.



## Configuration

The archive executor has the following configuration options:

- __file_cache_size__:  The number of file descriptors to be cached.
  You can improve overall performance by keeping files from being continuously opened and closed
  at each write.
- __file_cache_ttl_secs__:  The *Time To Live* of a file descriptor.  When this time reaches 0,
  the descriptor will be removed from the cache.
- __base_path__:  A directory on the file system where all logs are written.  Based on their type,
  rule Actions received from the Matcher can be logged in subdirectories of the base_path.
  However, the archive executor will only allow files to be written inside this folder.
- __default_path__:  A default path where all Actions that do not specify an `archive_type` in
  the payload are logged.
- __paths__:  A set of mappings from an archive_type to an `archive_path`, which is a subpath
  relative to the base_path.  The archive_path can contain variables, specified by the syntax
  `${parameter_name}`, which are replaced at runtime by the values in the Action's payload.

The archive path serves to decouple the type from the actual subpath, allowing you to write Action
rules without worrying about having to modify them if you later change the directory structure or
destination paths.

As an example of how an archive_path is computed, suppose we have the following configuration:

```toml
base_path =  "/tmp"
default_path = "/default/out.log"
file_cache_size = 10
file_cache_ttl_secs = 1

[paths]
"type_one" = "/dir_one/file.log"
"type_two" = "/dir_two/${hostname}/file.log"
```

and these three incoming actions:

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

  - _action_one_ will be archived in __/tmp/dir_one/file.log__
  - _action_two_ will be archived in __/tmp/dir_two/net-test/file.log__
  - _action_three_ will be archived in __/tmp/default/out.log__



## How it Works

The archive executor expects an Action to include the following elements in the payload:

1. An __event__:  The Event to be archived should be included in the payload under the key `event`.
1. An __archive type__ (optional):  The archive type is specified in the payload under the key
   `archive_type`.

When an archive_type is not specified, the default_path is used (as in action_three).  Otherwise,
the executor will use the archive_path in the `paths` configuration corresponding to the
`archive_type` key (action_one and action_two).

When an archive_type is specified but there is no corresponding key in the mappings under the
`paths` configuration, or it is not possible to resolve all path parameters, then the Event
will not be archived.   Instead, the archiver will return an error.

The Event from the payload is written into the log file in JSON format, one event per line.
