# JMESPath Collector

This is a Collector that receives an input in JSON format and allows the creation of Events using
the [JMESPath JSON query language](http://jmespath.org/).



## Configuration

The Collector configuration is composed of two named values:
- __event_type__:  Identifies the type of Event, and can be a String or a JMESPath expression (see below).
- __payload__:  A Map<String, ValueProcessor> with event-specific data.

and here the payload __ValueProcessor__ can be one of:
- A __null__ value
- A __string__
- A __bool__ value (i.e., true or false)
- A __number__
- An __array__ of values
- A __map__ of type Map<String, ValueProcessor>
- A __JMESPath expression__ :  A valid JMESPath expression delimited by the '${' and '}' characters.

The Collector configuration defines the structure of the Event produced. The configuration's
*event_type* property will define the type of Event, while the Event's *payload* will have the
same structure as the configuration's payload.



## How it Works

The __JMESPath expressions__ of the configuration will be applied to incoming inputs,
and the results will be included in the Event produced. All other __ValueProcessors__,
instead, are copied without modification.

For example, consider the following configuration:
```json
{
    "event_type": "webhook",
    "payload": {
        "name" : "${reference.authors[0]}",
        "from": "jmespath-collector",
        "active": true
    }
}
```

The value _${reference.authors[0]}_ is a JMESPath expression, delimited by `${` and `}`,
and whose value depends on the incoming input.


Thus if this input is received:
```json
{
    "date": "today",
    "reference": {
        "authors" : [
          "Francesco",
          "Thomas"
        ]
    }
}
```

then the Collector will produce this Event:
```json
{
    "event_type": "webhook",
    "payload": {
        "name" : "Francesco",
        "from": "jmespath-collector",
        "active": true
    }
}
```



## Runtime behavior

When the JMESPath expression returns an array or a map, the entire object will be inserted as-is
into the Event.

However, if a JMESPath expression does not return a valid result, then no Event is created, and
an error is produced.
