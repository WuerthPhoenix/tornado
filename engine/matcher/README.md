# Matcher Engine

The *tornado_engine_matcher* crate contains the core functions of the Tornado Engine. 
It defines the logic for parsing Rules and Filters as well as for matching Events.

  * [ ] The Matcher implementation details are [available here](./implementation.md).


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

The configuration of the processing tree stored on the file system in *json* format; when it is read
to be processed, the filter and rule names are automatically inferred from the filename and the node
names from the directory names.

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
  It can be composed only of letters, numbers and the "_" (underscore)
  character; it corresponds to the filename, stripped from its *.json* extension.
- `description`:  A string providing a high-level description of the filter.
- `active`:  A boolean value; if `false`, the filter's children will be ignored.
- `filter`:  A boolean operator that, when applied to an event, returns `true` or `false`.
  This operator determines whether an __Event__ matches the __Filter__; consequently, 
  it determines whether an __Event__ will be processed by the filter's inner nodes.


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

- __WHERE__:  A set of operators that when applied to an event returns `true` or `false`
- __WITH__:  A set of regular expressions that extract values from an Event and associate them
  with named variables

An event matches a rule if and only if the WHERE clause evaluates to `true` and all regular
expressions in the WITH clause return non-empty values.

The following operators are available in the __WHERE__ clause. Check also the examples in the
remainder of this document to see how to use them.
- __'contains'__: Evaluates whether the first argument contains the second one. It can be applied to
  strings, arrays, and maps. The operator can also be called with the alias __'contain'__.
- __'containsIgnoreCase'__: Evaluates whether the first argument contains, in a case-insensitive
  way, the **string** passed as second argument. This operator can also be called with the alias __'containIgnoreCase'__.
- __'equals'__: Compares any two values (including. but not limited to, arrays, maps) and returns
  whether or not they are equal. An alias for this operator is '__equal__'.
- __'equalsIgnoreCase'__:  Compares two strings and returns whether or not they are equal in a case-insensitive way.
  The operator can also be called with the alias __'equalIgnoreCase'__.
- __'ge'__:  Compares two values and returns whether the first value is greater than or equal 
  to the second one. If one or both of the values do not exist, it returns `false`.
- __'gt'__:  Compares two values and returns whether the first value is greater 
  than the second one. If one or both of the values do not exist, it returns `false`.
- __'le'__:  Compares two values and returns whether the first value is less than or equal 
  to the second one. If one or both of the values do not exist, it returns `false`.
- __'lt'__:  Compares two values and returns whether the first value is less 
  than the second one. If one or both of the values do not exist, it returns `false`.
- __'ne'__:  This is the negation of the __'equals'__ operator. Compares two values and returns whether 
  or not they are different. It can also be called with the aliases __'notEquals'__ and __'notEqual'__.
- __'regex'__:  Evaluates whether a field of an event matches a given regular expression.
- __'AND'__:  Receives an array of operator clauses and returns `true` if and only if all of them
  evaluate to `true`.
- __'OR'__:  Receives an array of operator clauses and returns `true` if at least one of the
  operators evaluates to `true`.
- __'NOT'__: Receives one operator clause and returns `true` if the operator clause evaluates to
  `false`, while it returns `false` if the operator clause evaluates to `true`.

We use the Rust Regex library (see its [github project home page](https://github.com/rust-lang/regex) )
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

The rule can access its fields as follows:
- `${event.type}`:  Returns **trap**
- `${event.payload.protocol}`:  Returns **UDP**
- `${event.payload.oids."key.with.dots"}`:  Returns **38:10:38:30.98**
- `${event.payload}`:  Returns the entire payload
- `${event}`: Returns the entire event


### String interpolation

An action payload can also contain text with placeholders that Tornado will replace at runtime.  The
values to be used for the substitution are extracted from the incoming _Events_ following the
conventions mentioned in the previous section; for example, using that Event definition, this string
in the action payload
 
`Received a ${event.type} with protocol ${event.payload.protocol}`
 
produces:
 
*Received a trap with protocol UDP*

> ### Note.

> Only values of type _String_, _Number_, _Boolean_ and _null_ are valid.  Consequently, the
> interpolation will fail, and the action will not be executed, if the value associated with the
> placeholder extracted from the Event is an _Array_, a _Map_, or _undefined_.

## Example of Filters


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

Therefore, with this configuration, the rules defined in *email/ruleset* receive only email events,
while those in *trapd/ruleset* receive only trapd events.

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
    "type": "equals",
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
    "type": "equals",
    "first": "${event.type}",
    "second": "trapd"
  }
}
```


## Examples of Rules and operators


### The 'contains' Operator

The _contains_ operator is used to check whether the first argument contains the second one.

It applies in three different situations:
- The arguments are both strings:  Returns true if the second string is a substring of the first one.
- The first argument is an array:  Returns true if the second argument is contained in the array.
- The first argument is a map and the second is a string:
  Returns true if the second argument is an existing key in the map.

In any other case, it will return false.

Rule example:
```json
{
  "description": "",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "contains",
      "first": "${event.payload.hostname}",
      "second": "linux"
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if in its payload appears an entry with key **hostname** and whose value is
a string that contains **linux**.

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

### The 'containsIgnoreCase' Operator

The _containsIgnoreCase_ operator is used to check whether the first argument contains the
**string** passed as second argument, regardless of their capital and small letters. In other words,
the arguments are compared in a *case-insensitive* way.

It applies in three different situations:
- The arguments are both strings:  Returns true if the second string is a _case-insensitive
  substring_ of the first one
- The first argument is an array: Returns true if the array passed as first parameter contains a
 (string) element which is equal to the string passed as second argument, regardless of uppercase
 and lowercase letters
- The first argument is a map: Returns true if the second argument contains, an existing,
  _case-insensitive_, key of the map

In any other case, this operator will return false.

Rule example:
```json
{
  "description": "",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "containsIgnoreCase",
      "first": "${event.payload.hostname}",
      "second": "Linux"
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if in its payload it has an entry with key "hostname" and whose value is
a string that contains "linux", __ignoring the case__ of the strings. 

A matching Event is:
```json
{
    "type": "trap",
    "created_ms": 1554130814854,
    "payload":{
        "hostname": "LINUX-server-01"
    }
}
```
Additional values for *hostname* that match the rule include: **linuX-SERVER-02**,
**LInux-Host-12**,  **Old-LiNuX-FileServer**, and so on.


### The 'equals', 'ge', 'gt', 'le', 'lt' and 'ne' Operators

The _equals_, _ge_, _gt_, _le_, _lt_, _ne_ operators are used to compare two values.

All these operators can work with values of type Number, String, Bool, null and Array. 

> ### Warning! 
>
> Please be extremely careful when using these operators with numbers of type **float**. The
> representation of floating point numbers is often slightly imprecise and can lead to unexpected
> results (for example, see: https://www.floating-point-gui.de/errors/comparison/).

Example:
```json
{
  "description": "",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "OR",
      "operators": [
        {
          "type": "equals",
          "first": "${event.payload.value}",
          "second": 1000
        },
        {
          "type": "AND",
          "operators": [
            {
              "type": "ge",
              "first": "${event.payload.value}",
              "second": 100
            },
            {
              "type": "le",
              "first": "${event.payload.value}",
              "second": 200
            },
            {
              "type": "ne",
              "first": "${event.payload.value}",
              "second": 150
            },
            {
              "type": "notEquals",
              "first": "${event.payload.value}",
              "second": 160
            }
          ]
        },
        {
          "type": "lt",
          "first": "${event.payload.value}",
          "second": 0
        },
        {
          "type": "gt",
          "first": "${event.payload.value}",
          "second": 2000
        }
      ]
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if _event.payload.value_ exists and one or more of the following
conditions hold:
- It is equal to _1000_
- It is between _100_ (inclusive) and _200_ (inclusive), but not equal to _150_ or to _160_
- It is less than _0_ (exclusive)
- It is greater than _2000_ (exclusive)

A matching Event is:
```json
{
    "type": "email",
    "created_ms": 1554130814854,
    "payload":{
      "value": 110
    }
}
```

Here are some examples showing how these operators behave:
- `[{"id":557}, {"one":"two"}]` _lt_ `3`: _false_
  (cannot compare different types, e.g. here the first is an array and the second is a number)
- `{id: "one"}` _lt_ `{id: "two"}`: _false_ (maps cannot be compared)
- `[["id",557], ["one"]]` _gt_ `[["id",555], ["two"]]`: _true_
  (elements in the array are compared recursively from left to right:  so here "id" is first compared to
  "id", then 557 to 555, returning true before attempting to match "one" and "two")
- `[["id",557]]` _gt_ `[["id",555], ["two"]]`: _true_
  (elements are compared even if the length of the arrays is not the same)
- `true` _gt_ `false`: _true_ (the value 'true' is evaluated as 1, and the value
  'false' as 0; consequently, the expression is equivalent to "1 gt 0" which is true)
- "twelve" _gt_ "two": _false_ (strings are compared lexically, and 'e' comes before
  'o', not after it) 

### The 'equalsIgnoreCase' Operator

The _equalsIgnoreCase_ operator is used to check whether the strings passed as arguments are equal in a
_case-insensitive_ way.

It applies **only if** both the first and the second arguments are strings. In any other case, the
operator will return false.

Rule example:
```json
{
  "description": "",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "equalsIgnoreCase",
      "first": "${event.payload.hostname}",
      "second": "Linux"
    },
    "WITH": {}
  },
  "actions": []
}
```
An event matches this rule if in its payload it has an entry with key "hostname" and whose value is
a string that is equal to "linux", __ignoring the case__ of the strings.

A matching Event is:
```json
{
    "type": "trap",
    "created_ms": 1554130814854,
    "payload":{
        "hostname": "LINUX"
    }
}
```

### The 'regex' Operator

The _regex_ operator is used to check if a string matches a regular expression.
The evaluation is performed with the Rust Regex library
(see its [github project home page](https://github.com/rust-lang/regex) )


Rule example:
```json
{
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


### The 'AND', 'OR', and 'NOT' Operators

The _and_ and _or_ operators work on a set of operators, while the _not_ operator
works on one single operator.
They can be nested recursively to define complex matching rules.

As you would expect:
- The _and_ operator evaluates to true if all inner operators match
- The _or_ operator evaluates to true if at least an inner operator matches
- The _not_ operator evaluates to true if the inner operator does not match,
  and evaluates to false if the inner operator matches


Example:
```json
{
  "description": "",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "AND",
      "operators": [
        {
          "type": "equals",
          "first": "${event.type}",
          "second": "rsyslog"
        },
        {
          "type": "OR",
          "operators": [
            {
              "type": "equals",
              "first": "${event.payload.body}",
              "second": "something"
            },
            {
              "type": "equals",
              "first": "${event.payload.body}",
              "second": "other"
            }
          ]
        },
        {
          "type": "NOT",
          "operator": {
              "type": "equals",
              "first": "${event.payload.body}",
              "second": "forbidden"
          }
        }
      ]
    },
    "WITH": {}
  },
  "actions": []
}

```
An event matches this rule if in its payload:
- The type is "rsyslog"
- __AND__ an entry with key *body* whose value is wither "something" __OR__ "other"
- __AND__ an entry with key *body*  is __NOT__  "forbidden"


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

All variables declared by a Rule must be resolved, or else the Rule will not be matched.

Two simple rules restrict the access and use of the extracted variables:
1. Because they are evaluated after the _WHERE_ clause is parsed, any extracted variables declared
   inside the _WITH_ clause are not accessible by the _WHERE_ clause of the very same rule
2. A rule can use extracted variables declared by other rules, even in its _WHERE_ clause, provided
   that:
   - The two rules must belong to the same rule set
   - The rule attempting to use those variables should be executed after the one that declares them
   - The rule that declares the variables should also match the event

The syntax for accessing an extracted variable has the form:

**_variables.**[*.RULE_NAME*].*VARIABLES_NAME*

If the *RULE_NAME* is omitted, the current rule name is automatically selected.


Example:
```json
{
  "description": "",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
          "type": "equals",
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

### The 'WITH' Clause - Configuration details

As already seen in the previous section, the _WITH_ clause generates 
variables extracted from the Event using regular expressions.
There are multiple ways of configuring those regexes
to obtain the desired result.

Essentially, three parameters combined will define the behavior of an extractor:
- **all_matches**: whether the regex will loop through all the matches or only the first one will be considered. 
Accepted values are _true_ and _false_. If omitted, it defaults to _false_
- **match** or **named_match**: a string value representing the regex to be executed. **match** is
used in case of an index-based regex, while **named_match** is used when named groups are
present. Note that they are mutually exclusive.
- **group_match_idx**: valid only in case of an index-based regex.
It is a positive numeric value that indicates which group of the match has to be extracted.
If omitted, an array with **all** groups is returned.

To show how they work and what is the produced output, from now on, we'll use this hypotetical email body as input:
```
A critical event has been received:

STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3
STATUS: OK HOSTNAME: MYHOST SERVICENAME: MYVALUE41231
```

Our objective is to extract from it information about the host status and name, and the service
name. We show how using different extractors leads to different results.

**Option 1** 
```json
"WITH": {
      "server_info": {
        "from": "${event.payload.email.body}",
        "regex": {
          "all_matches": false,
          "match": "STATUS:\s+(.*)\s+HOSTNAME:\s+(.*)SERVICENAME:\s+(.*)",
          "group_match_idx": 1
        }
      }
    }
```
This extractor:
- processes only the first match because **all_matches** is _false_
- uses an index-based regex specified by **match**
- returns the group of index **1**

In this case the output will be the string _"CRITICAL"_.

Please note that, if the *group_match_idx* was 0, it would have returned 
*"STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3"* as in
any regex the group with index 0 always represents the full match.

**Option 2**
```json
"WITH": {
      "server_info": {
        "from": "${event.payload.email.body}",
        "regex": {
          "all_matches": false,
          "match": "STATUS:\s+(.*)\s+HOSTNAME:\s+(.*)SERVICENAME:\s+(.*)"
        }
      }
    }
```
This extractor:
- processes only the first match because **all_matches** is _false_
- uses an index-based regex specified by **match**
- returns an array with **all** groups of the match
because *group_match_idx* is omitted.

In this case the output will be an array of strings:
```
[
  "STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3",
  "CRITICAL",
  "MYVALUE2",
  "MYVALUE3"
]
```

**Option 3**
```json
"WITH": {
      "server_info": {
        "from": "${event.payload.email.body}",
        "regex": {
          "all_matches": true,
          "match": "STATUS:\s+(.*)\s+HOSTNAME:\s+(.*)SERVICENAME:\s+(.*)",
          "group_match_idx": 2
        }
      }
    }
```
This extractor:
- processes all matches because **all_matches** is _true_
- uses an index-based regex specified by **match**
- for each match, returns the group of index **2**

In this case the output will be an array of strings:
```
[
  "MYVALUE2", <-- group of index 2 of the first match
  "MYHOST"    <-- group of index 2 of the second match
]
```

**Option 4**
```json
"WITH": {
      "server_info": {
        "from": "${event.payload.email.body}",
        "regex": {
          "all_matches": true,
          "match": "STATUS:\s+(.*)\s+HOSTNAME:\s+(.*)SERVICENAME:\s+(.*)"
        }
      }
    }
```
This extractor:
- processes all matches because **all_matches** is _true_
- uses an index-based regex specified by **match**
- for each match, returns an array with **all** groups of the match
because *group_match_idx* is omitted.

In this case the output will be an array of arrays of strings:
```
[
  [
    "STATUS: CRITICAL HOSTNAME: MYVALUE2 SERVICENAME: MYVALUE3",
    "CRITICAL",
    "MYVALUE2",
    "MYVALUE3"
  ],
  [
    "STATUS: OK HOSTNAME: MYHOST SERVICENAME: MYVALUE41231",
    "OK",
    "MYHOST",
    "MYVALUE41231"
  ]
]
```

The inner array, in position 0, contains all the groups of the first match
while the one in position 1 contains the groups of the second match.

**Option 5**
```json
"WITH": {
      "server_info": {
        "from": "${event.payload.email.body}",
        "regex": {
          "named_match": "STATUS:\s+(?P<STATUS>.*)\s+HOSTNAME:\s+(?P<HOSTNAME>.*)SERVICENAME:\s+(?P<SERVICENAME>.*)"
        }
      }
    }
```
This extractor:
- processes only the first match because **all_matches** is omitted
- uses a regex with named groups specified by **named_match**

In this case the output is an object where the 
group names are the property keys:
```
{
  "STATUS": "CRITICAL",
  "HOSTNAME": "MYVALUE2",
  "SERVICENAME: "MYVALUE3"
}
```

**Option 6**
```json
"WITH": {
      "server_info": {
        "from": "${event.payload.email.body}",
        "regex": {
          "all_matches": true,
          "named_match": "STATUS:\s+(?P<STATUS>.*)\s+HOSTNAME:\s+(?P<HOSTNAME>.*)SERVICENAME:\s+(?P<SERVICENAME>.*)"
        }
      }
    }
```
This extractor:
- processes all matches because **all_matches** is _true_
- uses a regex with named groups specified by **named_match**

In this case the output is an array that contains one object
for each match:
```
[
  {
    "STATUS": "CRITICAL",
    "HOSTNAME": "MYVALUE2",
    "SERVICENAME: "MYVALUE3"
  },
  {
    "STATUS": "OK",
    "HOSTNAME": "MYHOST",
    "SERVICENAME: "MYVALUE41231"
  },
]
```


### Complete Rule Example 1

An example of a valid Rule in a JSON file is:
```json
{
  "description": "This matches all emails containing a temperature measurement.",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "AND",
      "operators": [
        {
          "type": "equals",
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
        "temperature:": "The temperature is: ${_variables.temperature} degrees"
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
