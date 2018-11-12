# Common API

The common API module contains the Trait definitions for cross-component communication.
Three main components are currently defined: 
- the Collectors 
- the Tornado Engine 
- the Executors

The Tornado Engine receives Events from the collectors, processes them against a set of rules and triggers the Actions defined on the matching rules sending messages to the appropriate executors.

# Event
An event has a simple structure, it is composed of:

- __type__: the Event type identifier. Usually, a collector always sends Events of the same type;
- __created_ts__: the Event creation timestamp in nanoseconds;
- __payload__: a Map<String, String> with event specific data.

All fields are mandatory.
The payload is required but it can be empty.

Example of an Event in JSON format:
```json
{
    "type": "email",
    "created_ts": 12345678123123123, 
    "payload": {
        "subject" : "Doing something",
        "body": "everything's done"
    }
}
```

# Action
As for the Event, the structure of an Action is really simple:

- __id__: the Action type identifier. Usually, an executor always processed a single type of action;
- __payload__: a Map<String, String> with action specific data.

All fields are mandatory. 
The payload is required but it can be empty.

Example of an Action in JSON format:
```json
{
    "id": "Monitoring",
    "payload" : {
        "host": "neteye.local",
        "service": "PING",
        "state": "CRITICAL",
        "comment": "42 Degrees"
    }
}

```