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



## Tornado Architecture  (Add architecture figure?)

The principal data types of Tornado are:
* Incoming Events
* Rules, that have (defined in next section):
    * Matching conditions
    * Definable variables 
    * Actions to be executed

On startup, Tornado's Configuration Parser reads stored rule configurations and converts them into
internal rule objects.  Rules are composable, and are required to have unique names, unique
priorities, and thus a strong ordering.  They are written in JSON (see Figure X for an example).

Architecturally, Tornado is organized as a processing pipeline, where input events move from
collectors to the rule engine, to executors, without branching or returning.  This pipeline
architecture greatly contributes to its speed.  The principal modules are:
* Datasources:  Original sources of events, typically applications or hardware, where different
    event types have different communication patterns.
    * Channel subscriptions for streamed events (e.g., Syslog, SNMP traps, DNS) or via NATS (e.g., monitoring, or Telegram)
    * Polling / Call (e.g., Email)
    * Direct read (e.g., SMS)
    * API call (e.g., AWS, Azure)
* Event Collectors:  Listens for events from a datasource and rewrites them into a standard format
  (payload?) that can be used by the Matcher.
* Rule Engine Matcher:  Compares the rewritten event against the pre-configured rule set in
  priority order until it finds a matching rule.
* Rule Engine Extractor/Dispatcher:  Once a matching rule is found, it creates variables from both
  the event payload and the rule definition, then sending it to the appropriate Tornado Executor.
* Action Executors:  Instantiates the variables into an action template and invokes that action.

Tornado is implemented in Rust, so that it is fully compiled and thus blazingly fast, is both
thread-safe and memory safe, and has excellent error handling.  By using Rust, Tornado can receive
hundreds of thousands of events per second and apply millions of rules per second.



## Tornado Configuration and Rules

* Tornado is aware when its configuration changes and will automatically reload it.
* Location of configuration files
* How Tornado should be configured internally
* How it should be configured within NetEye?
* A very specific quick start with example rule (Hello world?)
    * WHERE conditions must be true for the rule to match
    * WHERE conditions can be combined with AND and OR operators
    * WITH constraints enrich events with additional fields
    * WITH constraints must result in a valid match for the rule to match
    * WITH constraints are for now based on Regular Expressions

Is the following rule to big?  Can it be shortened?

{
    "name": "emails_with_temperature",
    "description": "This matches all emails",
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



## Navigating Tornado's Code and Documentation

* Navigating the documentation/code (searchable API?)
    * Description of the code structure
    * Link to set of links to documentation
    * Link to changelog
* How to compile and execute Tornado
    * Current Tornado version
    * How to download
    * How to install from source
        * NETEYE:  yum install tornado --enablerepo=neteye-extras
        * Building the source
        * Common installation/build problems
    * External requirements and dependencies
        * The expected environment
    * The executable to call
        * systemctl start tornado.service



## The Tornado Project

* Near term plans
  Finish the system
* Long term plans
  Add additional datasources, collectors and executors
  A graphical configuration interface for IcingaWeb2
* We follow semantic versioning
* License, support?   Fully open-sourced
* How to contribute to the project (is Github the focal point?)
    * Bug reports, requesting features, contributing code
    * Forum / Feedback channels?
    * Check that a bug hasn't already been reported?



## Should the content of these be included here?

- [Global architecture](doc/architecture.md)
- [Implementation details](doc/implementation.md)



## Crate Docs



### Common Traits and Code
- [tornado_common_api](src/common/api/doc/README.md)
- [tornado_common_logger](src/common/logger/doc/README.md)



### Collectors
- [tornado_collector_common](src/collector/common/doc/README.md)
- [tornado_collector_jmespath](src/collector/jmespath/doc/README.md)
- [tornado_collector_json](src/collector/json/doc/README.md)
- [tornado_collector_snmptrapd](src/collector/snmptrapd/doc/README.md)



### Engine
- [tornado_engine_matcher](src/engine/matcher/doc/README.md)



### Executors
- [tornado_executor_archive](src/executor/archive/doc/README.md)
- [tornado_executor_common](src/executor/common/doc/README.md)
- [tornado_executor_logger](src/executor/logger/doc/README.md)
- [tornado_executor_script](src/executor/script/doc/README.md)



### Network
- [tornado_network_common](src/network/common/doc/README.md)
- [tornado_network_simple](src/network/simple/doc/README.md)



## Tornado Executable Docs



### Common code
- [tornado_common](src/tornado/common/doc/README.md)



### Executables
- [tornado](src/tornado/tornado/doc/README.md)
- [tornado_rsyslog_collector](src/tornado/rsyslog_collector/doc/README.md)
- [tornado_webhook_collector](src/tornado/webhook_collector/doc/README.md)

