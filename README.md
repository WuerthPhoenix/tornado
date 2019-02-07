# Tornado Basics

Tornado is a Complex Event Processor that receives reports of events from data sources such as
monitoring, email, and telegram, matches them against pre-configured rules, and executes the
actions associated with those rules, which can include notifications, logging, and graphing.

Tornado is a high performance, scalable, and multi-tenant capable application based on
communications secured with certificates.  It is intended to handle hundreds of thousands
of events each second on standard server hardware.

When Tornado receives an event, a dedicated collector for that specific event type converts
it into a JSON structure which can be matched against user-defined, composable rules.  Collectors
for new event types can be easily extended from existing types:
* Monitoring events
* Email messages
* Telegram
* DNS
* Cloud monitoring (AWS, Azure, Cisco/Meraki, etc.)
* Netflow
* Elastic Stack
* SMS
* SNMP
* Operating system and authorization events

Because all collectors and rules are defined with a standard format in JSON, the matching engine
can be simplified.  Matched events can potentially trigger multiple rules, whose actions can
include:
* Forwarding the events to a monitoring system
* Logging events locally (e.g., as processed, discarded or matched)
* Archiving events using an application such as Elastic Stack
* Invoking a custom shell script
* Crafting a new event that will be sent to a collector



## Tornado Architecture

<!-- Add an architecture diagram? -->

The principal data types of Tornado are:
* Incoming Events
* Rules, that have (as defined in next section):
    * Matching conditions
    * Definable variables 
    * Actions to be executed

On startup, Tornado's Configuration Parser reads stored rule configurations and converts them into
internal rule objects.  Rules are composable, written in JSON, and are required to have unique
names, unique priorities, and thus have a strong ordering.

Architecturally, Tornado is organized as a processing pipeline, where input events move from
collectors to the rule engine, to executors, without branching or returning.  This pipeline
architecture greatly contributes to its speed.  The principal modules are:
* Datasources:  Original sources of events, typically applications or hardware, where different
    event types have different communication patterns.
    * Channel subscriptions for streamed events (e.g., Syslog, SNMP traps, DNS) or via NATS (e.g., monitoring, or Telegram)
    * Polling / Call (e.g., Email)
    * Direct read (e.g., SMS)
    * API call (e.g., AWS, Azure)
* Event Collectors:  Listen for events from a datasource and rewrites them into a standard format
  (called a *payload*) that can be used by the Matcher.
* Rule Engine Matcher:  Compares the rewritten event against the pre-configured rule set in
  priority order until it finds a matching rule.
* Rule Engine Extractor/Dispatcher:  Once a matching rule is found, it creates variables from both
  the event payload and the rule definition, then sending it to the appropriate Tornado Executor.
* Action Executors:  Instantiates the variables into an action template and invokes that action.

Tornado is implemented in Rust, so it is fully compiled and thus blazingly fast, is both
thread-safe and memory safe, and has excellent error handling.  Because it uses Rust, Tornado
can receive hundreds of thousands of events per second and match millions of rules per second.

At the following links you can find more information about:
* [Tornado's architecture](doc/architecture.md).
* [Implementation details](doc/implementation.md)



## Tornado Configuration and Rules

Configuring Tornado requires the following steps:
* Configuring the Unix Domain Sockets (UDS) between the datasources and collectors
* Indicating the location for storing log files
* Creating the main configuration folder
* Configuring rules for your particular deployment

<!-- Is there a default configuration folder path? -->

<!-- Should we mention how to configure Tornado within NetEye? -->

Tornado monitors changes to its own configuration and will automatically reload it when this occurs.

Before you can begin to use Tornado, you must configure it with one or more rules that match
events and execute actions.  As an example, consider this rule below designed to find email
messages containing temperature measurements and log them in a standard, compressed form that
can easily be used by graphing software.  The rule contains the following fields:

* __Name:__  A unique name to differentiate this rule from others
* __Description:__  A human-readable description describing what the *constraint* and *actions* fields do
* __Priority:__  A unique priority allowing the matcher to first match high priority rules
* __Continue:__  Whether to keep matching additional rules if the current rule matches
* __Active:__  Whether this rule is currently enabled or disabled
* __Constraint:__  Consists of a single __WHERE__ clause to match the event, and a single __WITH__
  clause that extracts values as named variables to be used in the rule's action.

For a given rule to match, the evaluated WHERE expression (which can contain variables
pre-calculated by the Collector) must return `true` and all variables in the WITH clause
must return non-null values.  The WHERE expression can contain logical operators such as AND
and OR.  The WITH clause allows you to create new variables using regular expression matches on
the pre-calculated variables.

<!-- Can we shorten/improve the following rule? -->

Thus the following rule matches all email events (type "equal", second "email") where the
regular expression "[0-9]+\\sDegrees" matches the body of the email.  The rule is made more
efficient because events that are not of type "email" are discarded immediately before
an attempt is made at finding a more time-consuming regex match.

```json
{
    "name": "emails_with_temperature_measurements",
    "description": "Matches all emails containing ",
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
        "id": "Logger",
        "payload": {
            "type": "${event.type}",
            "subject": "${eventl.payload.subject}",
            "temperature": "${_variables.temperature}"
        }
    ]
}
```

If a match is made, the action clause indicates an event of type "email" should be logged with the
original subject line kept intact, and the extracted temperature stored as a numeric value that
can be processed separately.



## Compiling and Running Tornado

<!-- Should we include the NetEye instructions?  yum install tornado --enablerepo=neteye-extras -->


### Prerequisites
- Rust 1.32 or later should be installed, make sure you're using at least that version of Rust;
- At the moment, a Unix-like OS is required; this is due to the fact that UDS sockets are used for the communication between the various Tornado components;
- The openssl-dev library should be present in the build system.

### Build process

You can download and install the Tornado source for Linux by cloning from the .git repository.

<!-- TODO:  Add exact instructions once the real GitHub repository is online. -->

To build the source, open a shell where
you cloned the repository, change to the *src* directory, and type:
```
$ cargo build
```

This will build the entire project and produces a bunch of executable files in the *src/target/debug* folder.

You can, alternatively, perform a release build with:
```
$ cargo build --release
```
This will produce smaller highly optimized executable in the *src/target/release* folder. If you intend to benchmark, assess or deploy Tornado in a production environment, this is the way you should built it.

The issues of the Tornado build process can be grouped in three categories:
- The spikes: All the executables with suffix _spike-_ are the TODO
<!-- Does it print anything when running?  How can you tell it's working properly? -->
<!-- Do we want to include a section on common build/install problems? -->
<!-- Why are there more than one binary executables? -->
<!-- Do we want to list external requirements and dependencies? -->

### Run Tornado
TO DO

## The Tornado Project

<!-- Do we have a searchable API? -->

<!-- Where is our changelog? -->

Tornado is still in a beta phase, thus the next steps in its development are to finish the
remaining elements of the architecture.  Longer term, we plan to add additional datasources,
collectors and executors, and eventually create a graphical interface for rule configuration
and integration.

Tornado adheres to v2.0.0 of the [Semantic Versioning Initiative](http://semver.org/spec/v2.0.0.html).
It is fully open source with the official repository on [GitHub](link.html),
and available under the X license.

<!-- Do we need to mention Support as some other projects do? -->

You can contribute to Tornado by reporting bugs, requesting features, or contributing code
on GitHub.  If you intend to submit a bug, please check first that someone else has not already
submitted it by searching the issue tracker on GitHub.

<!-- Do we have a forum or other feedback channel?  If so, should we mention it? -->

Tornado's crate docs are produced according to
[Rust documentation standards](https://doc.rust-lang.org/book/index.html).
The shortcuts below, organized thematically, will take you to the documentation for each module.



### Common Traits and Code

The Common API page describes the API and defines the Event and Action structures.
- [tornado_common_api](src/common/api/doc/README.md)

The Logger page describes how Tornado logs its own actions.
- [tornado_common_logger](src/common/logger/doc/README.md)



### Collectors

This crate describes the commonalities of all Collector types.
<!-- This page of doc. is very short. -->
- [tornado_collector_common](src/collector/common/doc/README.md)

This page illustrates the Collector for JSON evevents using the JMESPath JSON query language.
- [tornado_collector_jmespath](src/collector/jmespath/doc/README.md)

Presents the standard JSON collector that deserializes an unstructured JSON string into an Event.
- [tornado_collector_json](src/collector/json/doc/README.md)

Describes an SNMP trap collector that receives *snmptrapd* messages formatted as JSON and generates 
an Event.
- [tornado_collector_snmptrapd](src/collector/snmptrapd/doc/README.md)



### Engine

The Matcher page describes the structure of the rules used in matching.
<!-- It doesn't describe anything else about the matcher besides the rule structure. -->
- [tornado_engine_matcher](src/engine/matcher/doc/README.md)



### Executors

This crate describes the commonalities of all Executor types.
<!-- This page of doc. is very short. -->
- [tornado_executor_common](src/executor/common/doc/README.md)

This page describes how the Archive executor writes to log files on locally mounted file systems,
with a focus on configuration.
- [tornado_executor_archive](src/executor/archive/doc/README.md)

The Logger executor simply outputs the whole Action body 
to the standard [log](https://crates.io/crates/log) at _info_ level.
<!-- This page of doc. is very short. -->
- [tornado_executor_logger](src/executor/logger/doc/README.md)

The Executor Script page defines how to configure Actions that launch shell scripts.
<!-- Has not been checked for English yet. -->
- [tornado_executor_script](src/executor/script/doc/README.md)



### Network

This page contains high level traits not bound to any specific network technology.
<!-- This page of doc. is very short. -->
- [tornado_network_common](src/network/common/doc/README.md)

Describes tests that dispatch Events and Actions on a single process without actually making network calls.
<!-- This page of doc. is very short. -->
- [tornado_network_simple](src/network/simple/doc/README.md)



## Documentation for Tornado Executables



### Common code

This page describes common structures and error handling, especially for UDS code from third
parties, such as Actix and Tokio.
<!-- README.md not yet in a branch? -->
- [tornado_common](src/tornado/common/doc/README.md)



### Executables

Describes the structure of the Tornado binary executable, and the structure and configuration of many of its components.
- [tornado](src/tornado/tornado/doc/README.md)

The description of a binary executable that generates Tornado Events from rsyslog inputs.
- [tornado_rsyslog_collector](src/tornado/rsyslog_collector/doc/README.md)

A standalone HTTP server binary executable that listens for REST calls from a generic webhook.
- [tornado_webhook_collector](src/tornado/webhook_collector/doc/README.md)
