# Matcher Engine

The *tornado_engine_matcher* crate contains the core functions of the Tornado Engine. It defines the logic to parse a
Rule as well as for matching Events and Rules.

Matcher implementation details are [available here](./implementation.md)

## Structure of a rule

A rule is composed of a set of properties, constraints and actions.


### Basic properties

- `rule name`:  A string value representing a unique rule identifier. It can be composed only of
  alphabetical characters, numbers and the "_" (underscore) character.
- `description`:  A string value providing a high-level description of the rule.
- `priority`:  A unique, positive integer that defines the execution order of the rules.
  '0' (zero) is the highest priority and denotes the first rule to be evaluated.
- `continue`:  A boolean value indicating whether to proceed with the event matching process if the current rule matches.
- `active`:  A boolean value; if `false`, the rule is ignored.



### Constraints

The constraint section contains the tests that determine whether or not an event matches the rule.
There are two types of constraints:

- __WHERE__:  A set of operators that when applied to an event returns `true` or `false`.
- __WITH__:  A set of regular expressions that extract values from an Event and associate them
  with named variables.

An event matches a rule if and only if the WHERE clause evaluates to `true` and all regular
expressions in the WITH clause return non-empty values.

The following operators are available in the __WHERE__ clause:
- __'contain'__: Evaluates whether a string contains a given substring.
- __'equal'__:  Compares two values and returns whether or not they are equal. If one or both of
  the values do not exist, it returns `false`.
- __'regex'__:  Evaluates whether a field of an event matches a given regular expression.
- __'AND'__:  Receives an array of operator clauses and returns `true` if and only if all of them
  evaluate to `true`.
- __'OR'__:  Receives an array of operator clauses and returns `true` if at least one of the
  operators evaluates to `true`.

We use the Rust Regex library (see its [github project here](https://github.com/rust-lang/regex) )
to evaluate regular expressions provided by the _WITH_ clause and by the _regex_ operator.
You can also refer to its [dedicated documentation](https://docs.rs/regex) for details about its
features and limitations.



### Actions

An Action is an operation triggered when an Event matches a Rule.



### Reading Event fields

A Rule can access Event fields through the "${" and "}" delimiters. To do so, the following
conventions are defined:
- The '.' (dot) char is used to access inner fields.
- Keys containing dots are escaped with leading and trailing double quotes.
- Double quote chars are not accepted inside a key.

For example, given the incoming event:
```json
{
    "type": "trap",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00",
    "payload":{
        "protocol": "UDP",
        "oids": {
            "key.with.dots": "38:10:38:30.98"
        }
    }
}
```

The following accessors are valid:
- `${event.type}`:  Returns "trap"
- `${event.payload.protocol}`:  Returns "UDP"
- `${event.payload.oids."key.with.dots"}`:  Returns "38:10:38:30.98"
- `${event.payload}`:  Returns the entire payload
- `${event}`: Returns the entire event


## Rule Examples

### The 'contain' operator
The _contain_ operator is used to check if a string contains a substring.

Rule example:
```json
{
  "name": "contain_operator",
  "description": "",
  "priority": 0,
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "contain",
      "text": "${event.payload.hostname}",
      "substring": "linux"
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if in its payload it has
an entry with key "hostname" and whose value is a string that contains
"linux".

A matching Event is:
```json
{
    "type": "trap",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00",
    "payload":{
        "hostname": "linux-server-01"
    }
}
```

### The 'equal' operator
The _equal_ operator is used to check if two values are the same.

Example:
```json
{
  "name": "equal_operator",
  "description": "",
  "priority": 0,
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "equal",
      "first": "${event.type}",
      "second": "email"
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if its type is "email".

A matching Event is:
```json
{
    "type": "email",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00",
    "payload":{}
}
```

### The 'regex' operator
The _regex_ operator is used to check if a string matches a regular expression.
The evaluation is performed with the Rust Regex library
(see its [github project here](https://github.com/rust-lang/regex) )


Rule example:
```json
{
  "name": "regex_operator",
  "description": "",
  "priority": 0,
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "regex",
      "regex": "[a-fA-F0-9]",
      "target": "${event.type}"
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if its type matches the regular expression [a-fA-F0-9].

A matching Event is:
```json
{
    "type": "trap0",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00",
    "payload":{}
}
```

### The 'and' and 'or' operator
The _and_ and _or_ operators work on a set of operators.
They can be nested recursively to define complex matching rules.

As you should expect:
- The _and_ operator evaluates to true if all inner operators match
- The _or_ operator evaluates to true if at least an inner operator matches


Example:
```json
{
  "name": "complex_rule",
  "description": "",
  "priority": 42,
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "AND",
      "operators": [
        {
          "type": "equal",
          "first": "${event.type}",
          "second": "rsyslog"
        },
        {
          "type": "OR",
          "operators": [
            {
              "type": "equal",
              "first": "${event.payload.body}",
              "second": "something"
            },
            {
              "type": "equal",
              "first": "${event.payload.body}",
              "second": "other"
            }
          ]
        }
      ]
    },
    "WITH": {}
  },
  "actions": []
}

```
An event matches this rule if:
- in its payload it has an entry with key "body" and whose value is "something" __OR__ "other"
- __AND__ its type is "rsyslog"

A matching Event is:
```json
{
    "type": "rsyslog",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00",
    "payload":{
        "body": "other"
    }
}
```


### A 'Match all Events' rule

If the _WHERE_ clause is not specified, the Rule evaluates to true for each incoming event.

For example, this Rule generates an "archive" Action for each Event:
```json
{
    "name": "rule_without_where",
    "description": "",
    "priority": 4,
    "continue": true,
    "active": true,
    "constraint": {
      "WITH": {}
    },
    "actions": [
      {
        "id": "archive",
        "payload": {
          "event": "${event}",
          "archive_type": "one"
        }
      }
    ]
}
```

### The 'WITH' clause
The _WITH_ clause generates variables extracted from the Event based on regular expressions.
These variables can then be used to populate an Action payload.

All variables declared by a Rule should be resolved, otherwise the Rule will not be matched.

Example:
```json
{
  "name": "motion_sensor_4",
  "description": "",
  "priority": 9,
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
          "type": "equal",
          "first": "${event.type}",
          "second": "trap"
    },
    "WITH": {
      "sensor_description": {
        "from": "${event.payload.line_5}",
        "regex": {
          "match": "(.*)",
          "group_match_idx": 0
        }
      },
      "sensor_room": {
        "from": "${event.payload.line_6}",
        "regex": {
          "match": "(.*)",
          "group_match_idx": 0
        }
      }
    }
  },
  "actions": [
    {
      "id": "nagios",
      "payload": {
        "host": "bz-outsideserverroom-sensors",
        "service": "motion_sensor_port_4",
        "status": "Critical",
        "host_ip": "${event.payload.host_ip}",
        "room": "${_variables.sensor_room}",
        "message": "${_variables.sensor_description}"
      }
    }
  ]
}

```

This Rules matches only if its type is "trap" and it is possible to extract the two variables
"sensor_description" and "sensor_room" defined by the _WITH_ clause.

An Event that matches this Rule is:
```json
{
  "type": "trap",
  "created_ts": "2018-11-28T21:45:59.324310806+09:00",
  "payload":{
    "host_ip": "10.65.5.31",
    "line_1":  "netsensor-outside-serverroom.wp.lan",
    "line_2":  "UDP: [10.62.5.31]:161->[10.62.5.115]",
    "line_3":  "DISMAN-EVENT-MIB::sysUpTimeInstance 38:10:38:30.98",
    "line_4":  "SNMPv2-MIB::snmpTrapOID.0 SNMPv2-SMI::enterprises.14848.0.5",
    "line_5":  "SNMPv2-SMI::enterprises.14848.2.1.1.7.0 38:10:38:30.98",
    "line_6":  "SNMPv2-SMI::enterprises.14848.2.1.1.2.0 \"Outside Server Room\""
  }
}
```

It will generate this Action:
```json
    {
      "id": "nagios",
      "payload": {
        "host": "bz-outsideserverroom-sensors",
        "service": "motion_sensor_port_4",
        "status": "Critical",
        "host_ip": "10.65.5.31",
        "room": "SNMPv2-SMI::enterprises.14848.2.1.1.7.0 38:10:38:30.98",
        "message": "SNMPv2-SMI::enterprises.14848.2.1.1.2.0 \"Outside Server Room\""
      }
    }
```

### Complete Rule Example 1

Example of valid content for a Rule JSON file is:
```json
{
  "name": "emails_with_temperature",
  "description": "This matches all emails containing a temperature measurement.",
  "priority": 2,
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "AND",
      "operators": [
        {
          "type": "equal",
          "first": "${event.type}",
          "second": "email"
        }
      ]
    },
    "WITH": {
      "temperature": {
        "from": "${event.payload.body}",
        "regex": {
          "match": "[0-9]+\\sDegrees",
          "group_match_idx": 0
        }
      }
    }
  },
  "actions": [
    {
      "id": "Logger",
      "payload": {
        "type": "${event.type}",
        "subject": "${event.payload.subject}",
        "temperature:": "${_variables.temperature}"
      }
    }
  ]
}
```

This creates a Rule with the following characteristics:
- Its unique name is 'emails_with_temperature'. There cannot be two rules with the same name.
- Its priority is 2. The priority defines the execution order of the rules:
  '0' (zero) is the highest priority and denotes the first rule to be evaluated.
- An Event matches this Rule if, as specified by the _WHERE_ clause, it has type "email", and,
  as requested by the _WITH_ clause, it is possible to extract the "temperature" variable from
  the "event.payload.body" with a non-null value.
- If an Event meets the previously stated requirements, the matcher produces an Action
  with _id_ "Logger" and a _payload_ with the three entries _type_, _subject_ and _temperature_.