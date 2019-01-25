# Common API

The *tornado_common_api* crate contains the API for cross-component communication.
Three main components are currently defined: 
- The Collectors 
- The Tornado Engine 
- The Executors

The Tornado Engine receives Events from the Collectors, checks them against a set of Rules, and
then triggers Actions defined on the Rules matched by sending messages to the appropriate executors.


# Event

An event has a simple structure, composed of:

- __type__:  The Event type identifier (a Collector usually sends Events of a single type)
- __created_ts__:  The Event creation timestamp in ISO 8601 format
- __payload__:  A Map<String, Value> with event-specific data

where the payload __Value__ can be any valid JSON type:
- A __string__
- A __bool__ value (i.e., true or false)
- A __number__ 
- An __array__ of Values
- A __map__ of type Map<String, Value>

All fields are mandatory, although the _payload_ can be an empty structure.

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

- __id__:  The Action type identifier (an Executor usually processes a single Action type)
- __payload__:  A Map<String, Value> with Action-specific data.

All fields are mandatory, although again the _payload_ can be an empty structure.

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