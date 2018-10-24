# Matcher

The matcher contains the core functions of the Tornado Engine.

It defines the logic to parse a Rule and the one for matching Events and Rules. 

## Structure of a rule
A rule is composed of a set of properties:

### Basic properties

- rule name: string value; unique rule identifier. It can be composed only of alphabetical characters, numbers and the "_" (underscore) char.
- description: string value; high-level description of the rule.
- priority: a positive integer; it defines the execution order of the rules. It must be unique. '0' (zero) is the highest priority and denotes the rule assessed first.
- continue: boolean value; whether to proceed with the event matching process if the current rule matches;
- active: boolean value; if false, the rule is ignored.

### Constraints

The constraint section contains the tests that determine whether an event matches the rule.
There are two types of constraint:

- WHERE: it is composed of a set of operators that applied to an event return true or false.
- WITH: it is composed of a set of regular expressions to extract values from an Event and associate them to named variables.

An event matches a rule if and only if the WHERE clause evaluates to true and all the regular expressions of the WITH clause return a non-empty value.

The following operators are available:
- 'equal': it compares two values and returns whether they are equal. If one or both the values do not exist, it returns false;
- 'regex': it evaluates if a field of an event matches a specific regular expression;
- 'AND': it receives an array of operators and returns true if all the operators evaluate to true.
- 'OR': it receives an array of operators and returns true if at least one of the operators evaluates to true.

### Actions

An action is an operation triggered when an event matches the rule.  