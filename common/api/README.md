# Common API

The *tornado_common_api* crate contains the API for cross-component communication. Three main
components are currently defined:
- The Collectors
- The Tornado Engine
- The Executors

The Tornado Engine receives Events from the Collectors, checks them against a set of Rules, and
then triggers Actions defined within the matched Rule(s) by sending messages to the appropriate
executors.



# Events

An Event has a simple structure, composed as follows:

- __trace_id__:  An optional unique Event identifier (usually a UUID). Tornado will generate a unique `trace_id` for each event that misses it.   
- __type__:  The Event type identifier (a given Collector usually sends Events of only a single type)
- __created_ms__:  The Event creation timestamp in milliseconds since January 1, 1970 UTC
- __payload__:  A Map<String, Value> with event-specific data

where the payload __Value__ can be any valid JSON type:
- A __null__ value
- A __string__
- A __bool__ value (i.e., true or false)
- A __number__
- An __array__ of Values
- A __map__ of type Map<String, Value>

All fields must have values, although the _payload_ can be an empty structure.

Example Event in JSON format:
```json
{
    "trace_id": "130b53f2-4e64-452a-b6c0-c88516b6e7c7",
    "type": "email",
    "created_ms": 1554130814854,
    "payload": {
        "subject" : "Doing something",
        "body": "everything's done"
    }
}
```



# Actions

Like the Event structure, the structure of an Action is simple:

- __id__:  The Action type identifier (a given Executor usually processes just a single Action type)
- __payload__:  A Map<String, Value> with data specific to its Action.

All fields must have values, although again the _payload_ can be an empty structure.

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
