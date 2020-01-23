# Foreach Executor

An Executor that loops through a set of data and executes a list of actions
for each entry.

## How it Works

The Foreach executor extracts all values from an array of elements and 
injects each value to a list of action under the *item* key.

It has two mandatory configuration entries in its payload:
- **target**: the array of elements
- **actions**: the array of action to execute  

For example, given this rule definition:
```json
{
  "name": "do_something_foreach_value",
  "description": "This uses a foreach loop",
  "continue": true,
  "active": true,
  "constraint": {
    "WITH": {}
  },
  "actions": [
    {
      "id": "foreach",
      "payload": {
        "target": "${event.payload.values}",
        "actions": [
          {
            "id": "logger",
            "payload": {
              "source": "${event.payload.source}",
              "value": "the value is ${item}"
            }
          },
          {
            "id": "archive",
            "payload": {
              "event": "${event}",
              "item_value": "${item}"
            }
          }
        ]
      }
    }
  ]
}
```

When an event with this payload is received:
```json
{
  "type": "some_event",
  "created_ms": 123456,
  "payload":{
    "values": ["ONE", "TWO", "THREE"],
    "source": "host_01"
  }
}
```

Then the **target** of the foreach action is the array `["ONE", "TWO", "THREE"]`; 
consequently, each one of the two inner actions is executed three times; 
the first time with _item_ = "ONE", then with _item_ = "TWO" and, finally, with _item_ = "THREE".
