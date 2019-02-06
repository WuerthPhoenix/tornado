# JMESPath Collector

A Collector that receives an input in JSON format and allows the creation of Events using the
[JMESPath JSON query language](http://jmespath.org/).

## Configuration
The Collector configuration is composed of:
- __event_type__: It identifies the type of Event, and can be a String or a JMESPath expression
  (see below).
- __payload__: A Map<String, ValueProcessor> with event-specific data.

where the payload __ValueProcessor__ can be:
- A __string__ 
- A __bool__ value (i.e., true or false)
- A __number__ 
- An __array__ of values
- A __map__ of type Map<String, ValueProcessor>
- A __JMESpath expression__ : A valid JMESpath expression delimited by the '${' and '}' characters.

The Collector configuration defines the structure of the Event produced.

The configuration's event_type property will define the type of Event.

The Event's payload will have the same structure as the configuration's payload.

The __JMESpath expressions__ of the configuration will be applied to incoming inputs, 
and the results will be included in the Event produced. All other __ValueProcessors__, 
instead, are copied without modifications.

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

The value _${reference.authors[0]}_ is a JMESpath expression, delimited by `${` and `}`, 
whose value depends on the incoming input.


If this input is received:
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

Then the Collector will produce the Event:
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

When the JMESpath expression returns an array or a map, 
the entire object will be inserted as-is into the Event.

However, if a JMESpath expression does not return a valid result, then no Event is created,
and an error is produced.
This happens, for example, when the expression points to a non-existing node in the input JSON.
