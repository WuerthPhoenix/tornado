# Matcher

The matcher contains the core functions of the Tornado Engine.

It defines the logic to parse a Rule and the one for matching Events and Rules. 

## Structure of a rule
A rule is composed of a set of properties:

### Basic properties

- __rule name__: string value; unique rule identifier. It can be composed only of alphabetical characters, numbers and the "_" (underscore) char.
- __description__: string value; high-level description of the rule.
- __priority__: a positive integer; it defines the execution order of the rules. It must be unique. '0' (zero) is the highest priority and denotes the rule assessed first.
- __continue__: boolean value; whether to proceed with the event matching process if the current rule matches;
- __active__: boolean value; if false, the rule is ignored.

### Constraints

The constraint section contains the tests that determine whether an event matches the rule.
There are two types of constraint:

- __WHERE__: it is composed of a set of operators that applied to an event return true or false.
- __WITH__: it is composed of a set of regular expressions to extract values from an Event and associate them to named variables.

An event matches a rule if and only if the WHERE clause evaluates to true and all the regular expressions of the WITH clause return a non-empty value.

The following operators are available:
- __'equal'__: it compares two values and returns whether they are equal. If one or both the values do not exist, it returns false;
- __'regex'__: it evaluates if a field of an event matches a specific regular expression;
- __'AND'__: it receives an array of operators and returns true if all the operators evaluate to true.
- __'OR'__: it receives an array of operators and returns true if at least one of the operators evaluates to true.

The regular expressions provided by the _WITH_ clause and by the _regex_ operator, are evaluated through the Rust Regex library (see: https://github.com/rust-lang/regex ). Please refer to the specific documentation (here: https://docs.rs/regex ) for details about features and limitations.  

### Actions

An action is an operation triggered when an event matches the rule.  

### Reading Event fields
A rule can access event fields through the "${" and "}" delimiters; the following conventions are defined:
- the '.' (dot) char is used to access inner fields;
- Keys containing dots are escaped with trailing and an ending double quotes;
- Double quote chars are not accepted in a key.

For example, given the incoming event:
```json
{
    "event_type": "trap",
    "created_ts": "2018-11-28T21:45:59.324310806+09:00",
    "payload":{
        "protocol": "UDP",
        "oids": {
            "key.with.dots": "38:10:38:30.98",
        }
    }
}
```

The following accessors are valid:
- `${event.type}` -> returns "trap"
- `${event.payload.protocol}` -> returns "UDP"
- `${event.payload.oids."key.with.dots"}` -> returns "38:10:38:30.98"
- `${event.payload}` -> returns the entire payload

 

