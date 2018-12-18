# Matcher

The matcher contains the core functions of the Tornado Engine.  It defines the logic to parse a
Rule as well as for matching Events and Rules. 



## Structure of a rule

A rule is composed of a set of properties, constraints and actions.



### Basic properties

- `rule name`:  A string value representing a unique rule identifier.  It can be composed only of
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
- __'equal'__:  Compares two values and returns whether or not they are equal.  If one or both of
  the values do not exist, it returns `false`.
- __'regex'__:  Evaluates whether a field of an event matches a given regular expression.
- __'AND'__:  Receives an array of operator clauses and returns `true` if and only if all of them
  evaluate to `true`.
- __'OR'__:  Receives an array of operator clauses and returns `true` if at least one of the
  operators evaluates to `true`.

We use the Rust Regex library (see its [github project](https://github.com/rust-lang/regex) here)
to evaluate regular expressions provided by the _WITH_ clause and by the _regex_ operator.
You can also refer to its [dedicated documentation](https://docs.rs/regex) for details about its
features and limitations.  



### Actions

An Action is an operation triggered when an Event matches a Rule.  



### Reading Event fields

A Rule can access Event fields through the "${" and "}" delimiters.  To do so, the following
conventions are defined:
- The '.' (dot) char is used to access inner fields.
- Keys containing dots are escaped with leading and trailing double quotes.
- Double quote chars are not accepted inside a key.

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
- `${event.type}`:  Returns "trap"
- `${event.payload.protocol}`:  Returns "UDP"
- `${event.payload.oids."key.with.dots"}`:  Returns "38:10:38:30.98"
- `${event.payload}`:  Returns the entire payload

 

