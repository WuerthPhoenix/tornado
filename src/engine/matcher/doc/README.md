# Matcher Engine

The *tornado_engine_matcher* crate contains the core functions of the Tornado Engine. 
It defines the logic for parsing Rules and Filters as well as for matching Events.

The Matcher implementation details are [available here](./implementation.md)


## The Processing Tree

The engine logic is defined by a processing tree with two types of nodes:
- __Filter__:  A node that contains a filter definition and a set of child nodes
- __Rule set__:  A leaf node that contains a set of __Rules__

A full example of a processing tree is:
```
root
  |- node_0
  |    |- rule_one
  |    \- rule_two
  |- node_1
  |    |- inner_node
  |    |    \- rule_one
  |    \- filter_two
  \- filter_one
``` 

All identifiers of the processing tree (i.e. rule names, filter names, and node names) can be
composed only of letters, numbers and the "_" (underscore) character.

When the configuration of the processing tree is read from the file system, the filter and rule
names are automatically inferred from the filename and the node names from the directory names.

In the tree above, the root node is of type __Filter__. In fact, it contains the definition of
a filter named *filter_one* and has two child nodes called *node_0* and *node_1*.
 
When the matcher receives an __Event__, it will first check if it matches the *filter_one* condition;
if it does, the matcher will proceed to evaluate its child nodes. If, instead, the filter condition
does not match, the process stops and those children are ignored.

A node's children are processed independently. Thus *node_0* and *node_1* will be processed in
isolation and each of them will be unaware of the existence and outcome of the other one.
This process logic is applied recursively to every node. 

In the above processing tree, *node_0* is a rule set, so when the node is processed, the matcher
will evaluate an __Event__ against each rule to determine which one matches and what __Actions__
are generated.

On the contrary, *node_1* is another __Filter__; in this case, the matcher will check if the
event verifies the filter condition in order to decide whether to process its internal nodes.


## Structure of a Filter

A __Filter__ contains these properties:

- `filter name`:  A string value representing a unique filter identifier. 
  It can be composed only of letters, numbers and the "_" (underscore) character.
- `description`:  A string value providing a high-level description of the filter.
- `active`:  A boolean value; if `false`, the filter's children will be ignored.
- `filter`:  An operator that, when applied to an event, returns `true` or `false`.
  This operator determines whether an __Event__ matches the __Filter__; consequently, 
  it determines whether an __Event__ will be processed by the filter's inner nodes.

When the configuration is read from the file system, the filter name is automatically inferred
from the filename by removing its '.json' extension.


### Implicit Filters

If a __Filter__ is omitted, Tornado will automatically infer an implicit filter that passes
through all __Events__. This feature allows for less boiler-plate code when a filter is only
required to blindly forward all __Events__ to the internal rule sets.
  
For example, if *filter_one.json* is a __Filter__ that allows all __Events__ to pass through,
then this processing tree:
```
root
  |- node_0
  |    |- ...
  |- node_1
  |    |- ...
  \- filter_one.json
``` 

is equivalent to:
```
root
  |- node_0
  |    |- ...
  \- node_1
       |- ...
``` 

Note that in the second tree we removed the *filter_one.json* file. In this case, Tornado will 
automatically generate an implicit filter for the *root* node, and all incoming __Events__ 
will be dispatched to each child node.



## Structure of a Rule

A __Rule__ is composed of a set of properties, constraints and actions.


### Basic Properties

- `rule name`:  A string value representing a unique rule identifier. It can be composed only of
  alphabetical characters, numbers and the "_" (underscore) character.
- `description`:  A string value providing a high-level description of the rule.
- `continue`:  A boolean value indicating whether to proceed with the event matching process if the current rule matches.
- `active`:  A boolean value; if `false`, the rule is ignored.

When the configuration is read from the file system, the rule name is automatically inferred
from the filename by removing the extension and everything that precedes the first
'_' (underscore) symbol. For example:
- _0001_rule_one.json_ -> 0001 determines the execution order, "rule_one" is the rule name
- _0010_rule_two.json_ -> 0010 determines the execution order, "rule_two" is the rule name 


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


### Reading Event Fields

A Rule can access Event fields through the "${" and "}" delimiters. To do so, the following
conventions are defined:
- The '.' (dot) char is used to access inner fields.
- Keys containing dots are escaped with leading and trailing double quotes.
- Double quote chars are not accepted inside a key.

For example, given the incoming event:
```json
{
    "type": "trap",
    "created_ms": 1554130814854,
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


## Filter Examples


### Using a Filter to Create Independent Pipelines

We can use __Filters__ to organize coherent set of __Rules__ into isolated pipelines.

In this example we will see how to create two independent pipelines, one that receives only
events with type 'email', and the other that receives only those with type 'trapd'. 

Our configuration directory will look like this:
```
rules.d
  |- email
  |    |- ruleset
  |    |     |- ... (all rules about emails here)
  |    \- only_email_filter.json
  |- trapd
  |    |- ruleset
  |    |     |- ... (all rules about trapds here)
  |    \- only_trapd_filter.json
  \- filter_all.json
``` 

This processing tree has a root filter *filter_all* that matches all events. We have also defined
two inner filters; the first, *only_email_filter*, only matches events of type 'email'. The other,
*only_trapd_filter*, matches just events of type 'trap'.

With this configuration, the rules defined in *email/ruleset* receive only email events, while
those in *trapd/ruleset* receive only trapd events.

This configuration can be further simplified by removing the *filter_all.json* file:
```
rules.d
  |- email
  |    |- ruleset
  |    |     |- ... (all rules about emails here)
  |    \- only_email_filter.json
  \- trapd
       |- ruleset
       |     |- ... (all rules about trapds here)
       \- only_trapd_filter.json
``` 
In this case, in fact, Tornado will generate an implicit filter for the root node and the
runtime behavior will not change.   

Below is the content of our JSON filter files.

Content of *filter_all.json* (if provided):
```json
{
  "description": "This filter allows every event",
  "active": true
}
```

Content of *only_email_filter.json*:
```json
{
  "description": "This filter allows events of type 'email'",
  "active": true,
  "filter": {
    "type": "equal",
    "first": "${event.type}",
    "second": "email"
  }
}
```

Content of *only_trapd_filter.json*:
```json
{
  "description": "This filter allows events of type 'trapd'",
  "active": true,
  "filter": {
    "type": "equal",
    "first": "${event.type}",
    "second": "trapd"
  }
}
```


## Rule Examples


### The 'contain' Operator

The _contain_ operator is used to check if a string contains a substring.

Rule example:
```json
{
  "name": "contain_operator",
  "description": "",
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
    "created_ms": 1554130814854,
    "payload":{
        "hostname": "linux-server-01"
    }
}
```


### The 'equal' Operator

The _equal_ operator is used to check if two values are the same.

Example:
```json
{
  "name": "equal_operator",
  "description": "",
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
    "created_ms": 1554130814854,
    "payload":{}
}
```


### The 'regex' Operator

The _regex_ operator is used to check if a string matches a regular expression.
The evaluation is performed with the Rust Regex library
(see its [github project here](https://github.com/rust-lang/regex) )


Rule example:
```json
{
  "name": "regex_operator",
  "description": "",
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
    "created_ms": 1554130814854,
    "payload":{}
}
```


### The 'and' And 'or' Operator

The _and_ and _or_ operators work on a set of operators.
They can be nested recursively to define complex matching rules.

As you would expect:
- The _and_ operator evaluates to true if all inner operators match
- The _or_ operator evaluates to true if at least an inner operator matches


Example:
```json
{
  "name": "complex_rule",
  "description": "",
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
    "created_ms": 1554130814854,
    "payload":{
        "body": "other"
    }
}
```


### A 'Match all Events' Rule

If the _WHERE_ clause is not specified, the Rule evaluates to true for each incoming event.

For example, this Rule generates an "archive" Action for each Event:
```json
{
    "name": "rule_without_where",
    "description": "",
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


### The 'WITH' Clause

The _WITH_ clause generates variables extracted from the Event based on regular expressions.
These variables can then be used to populate an Action payload.

All variables declared by a Rule should be resolved, otherwise the Rule will not be matched.

Example:
```json
{
  "name": "motion_sensor_4",
  "description": "",
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

This Rule matches only if its type is "trap" and it is possible to extract the two variables
"sensor_description" and "sensor_room" defined in the _WITH_ clause.

An Event that matches this Rule is:
```json
{
  "type": "trap",
  "created_ms": 1554130814854,
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

An example of valid content for a Rule JSON file is:
```json
{
  "name": "emails_with_temperature",
  "description": "This matches all emails containing a temperature measurement.",
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
- An Event matches this Rule if, as specified in the _WHERE_ clause, it has type "email", and
  as requested by the _WITH_ clause, it is possible to extract the "temperature" variable from
  the "event.payload.body" with a non-null value.
- If an Event meets the previously stated requirements, the matcher produces an Action
  with _id_ "Logger" and a _payload_ with the three entries _type_, _subject_ and _temperature_.
