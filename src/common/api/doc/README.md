# Common API

The common API module contains the Trait definitions for cross-component communication.
Three main components are currently defined: 
- The Collectors 
- The Tornado Engine 
- The Executors

The Tornado Engine receives Events from the collectors, checks them against a set of rules, and
triggers Actions defined on the Rules matched by sending messages to the appropriate executors.



# Event

An event has a simple structure, composed of:

- __type__:  The Event type identifier (a collector usually sends Events of a single type)
- __created_ts__:  The Event creation timestamp in ISO 8601 format
- __payload__:  A Map<String, String> with event-specific data

All fields are mandatory, although the payload can be an empty structure.

Example Event in JSON format:
```json
{
    "type": "email",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00", 
    "payload": {
        "subject" : "Doing something",
        "body": "everything's done"
    }
}
```



# Action

Like the Event structure, the structure of an Action is rather simple:

- __id__:  The Action type identifier (an executor usually processes a single action type)
- __payload__:  A Map<String, String> with action-specific data.

All fields are mandatory, although again the payload can be an empty structure.

Example Action in JSON format:
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