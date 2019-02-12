# Tornado Basics

Tornado is a Complex Event Processor that receives reports of events from data sources such as
monitoring, email, and telegram, matches them against pre-configured rules, and executes the
actions associated with those rules, which can include notifications, logging, and graphing.

Tornado is a high performance, scalable application.
It is intended to handle millions of events each second on standard server hardware.


## Tornado Architecture

The three main components of the Tornado architecture are:
* The *Tornado Collector(s)*, or *Collector(s)*
* The *Tornado Engine*, or *Engine*
* The *Tornado Executor(s)*, or *Executor(s)*

The term *Tornado* refers to the whole project or to a deployed system that includes 
all the components.

Along with the main components, these concepts are fundamental in the Tornado architecture:
* A *Datasource*: A system that sends *External Events* to Tornado;
  or a system to which Tornado subscribes to receive *External Events*
* An *External Event*: an input received from a datasource. Its format depends on its source. 
  Examples of this are events from Rsyslog.
* A *Tornado (or Internal) Event*: the Tornado specific Event format.
* A *Rule*: a group of conditions that an Internal Event has to 
  match to trigger a set of Actions
* An *Action*: An operation performed by Tornado usually on an external system.
  For example, writing to Elastic Search or setting a state 
  in a monitoring system.

Architecturally, Tornado is organized as a processing pipeline, where input events move from
collectors to the engine, to executors, without branching or returning.


When the system receives an *External event*, it first arrives at a *Collector* where is converted into a *Tornado Event*; then, it is forwarded to the *Tornado Engine*
where it is matched against user-defined, composable *Rules*. Finally,generated *Action* are dispatched to the *Executors*.


The Tornado pipeline:

    Datasources (e.g. Rsyslog)
      |
      | External Events
      |
      \-> Tornado Collectors 
            |
            | Tornado (or Internal) Events 
            |
            \-> Tornado Engine (matches based on Rules)
                  |
                  | Actions
                  |
                  \-> Tornado Executors (execute the Actions)


<!-- Add an architecture diagram? -->

### Collectors
The purpose of a *Collector* is to receive and convert external events into the 
internal Tornado Event structure and forward them to the Tornado Engine.

*Collectors* are *Datasource*-specific. For each datasource, there is at least one collector that knows how to manipulate the datasource Events and generate Tornado Events.

Out of the box, Torndato provides a bunch of Collectors for handling inputs 
from SNMPTRAPD, Rsyslog and generic Webhooks.

Because all Collectors are defined with a simple format, Collectors for new event types 
can be easily added or extended from existing types for:
* Monitoring events
* Email messages
* Telegram
* DNS
* Cloud monitoring (AWS, Azure, Cisco/Meraki, etc.)
* Netflow
* Elastic Stack
* SMS
* Operating system and authorization events

### Engine
The *Engine* is the second step of the pipeline.
It receives and processes the events produced by the *Collectors*.
The outcome of this processing is fully defined by an ordered set of *Rules*; each *Rule* in the set determines:
* the conditions a *Tornado Event* has to respect to match it
* the actions to be executed in case of matching

These Rules are parsed at startup from a configuration folder where they are stored in JSON format.

When an event matches one or more *Rules*, the Engine produces a set of *Actions* 
and forwards them to one or more *Executors*.

### Executors
The *Executors* are the last node of the Tornado pipeline.

They receive the *Actions* produced from the *Engine* and trigger the associated process.

An *Action* can be whatever command, process or operation.
For example it can include:
* Forwarding the events to a monitoring system
* Logging events locally (e.g., as processed, discarded or matched) or remotely
* Archiving events using an application such as Elastic Stack
* Invoking a custom shell script

An *Executor* usually takes care of a single *Action* type.

## Compiling and Running Tornado

### Prerequisites

- You must have Rust version 1.32 or later installed
- Event if the Tornado libraries are completely cross-platform, at the moment, 
  a Unix-like OS is required to build the Tornado executables as they use UDS sockets for communication
  between the various components
- To build the Tornado executables, the *openssl-dev* library should be present in your build environment

### Repository structure
The Tornado source code is modularized in a set of reusable, and mostly independent, libraries.

This repository contains both the code of the libraries and the code of the Tornado executables
that use them.

The way the Tornado executables are built is only one of the many possible approaches.

Structure of the repository:

    src
      |-- collector # The Collector libraries
      |     |-- common # Common code and traits for all collectors
      |     |-- ... 
      |-- common # Common interfaces and message definitions
      |     |-- api    # Global traits required by the engine, collectors and executors
      |     |-- logger # The logger configuration
      |-- executor # The Executor libraries
      |     |-- common # Common code and traits for all collectors
      |     |-- ...
      |-- engine # The Engine libraries
      |     |-- matcher # The core library of Tornado Engine. It contains the logic that evaluates whether 
      |     |             an Internal Event matches a rule and to trigger the related Actions.
      |-- network # An abstract service used by components to communicate with each other
      |     |-- common # Common code and traits for the network
      |     |-- ...
      |-- scripts # command line utilities (not required by Tornado)
      |-- spike # functional or technical experiment (not required by Tornado)
      |-- tornado # Tornado executables
      |     |-- common # Common code and traits for all executables
      |     |-- engine # The Tornado Engine executable with embedded Tornado Executors
      |     |-- rsyslog_collector # A Tornado Collector to handle rsyslog events
      |     |-- webhook_collector # A Tornado Collector to handle generic webhook events

### Build process

To build the source, open a shell where you cloned the repository, change to the *src* directory,
and launch:
```
$ cargo build
```

This will build the entire project and produces executable files in the *src/target/debug* folder.

Alternatively, you can perform a release build with:
```
$ cargo build --release
```

This will produce smaller, highly optimized executables in the *src/target/release* folder.
If you intend to run benchmarks, assess or deploy Tornado in a production environment, this is the
way you should built it.

The issues of the Tornado build process can be grouped in three categories:
- Tornado libraries: everything not in the the "spike" or "tornado" folder. 
These are common Rust libraries used by Tornado and 
that can be imported by other projects as well.
- Tornado executables: The crates on the "src/tornado" folder generate the Tornado executables. These are what
  you need to run and deploy Tornado. All these executables are suffixed with *tornado_*
- spikes: The crates on the "src/spike" folder generate executables suffixed with *spike_*. This are 
  experimental crates not part of the Tornado architecture.


### How to Run Tornado
To run Tornado follow the configuration instructions 
of the Tornado executables provided by the individual
documentation pages:
* [tornado_engine documentation](src/tornado/engine/doc/README.md)
* [tornado_rsyslog_collector documentation](src/tornado/rsyslog_collector/doc/README.md)
* [tornado_web_collector documentation](src/tornado/webhook_collector/doc/README.md)


### How to Run Tornado locally with cargo-make
To easily test Tornado locally, you can run it using cargo-make as [described here](./RUN_WITH_CARGO.md).


## The Tornado Project

<!-- Do we have a searchable API? -->

<!-- Where is our changelog? -->

Tornado is still in a beta phase, thus the next steps in its development are to finish the
remaining elements of the architecture.  Longer term, we plan to add additional,
collectors and executors, and eventually create a graphical interface for rule configuration
and integration.

Tornado is implemented in Rust and use no unsafe code. 
It is blazingly fast, thread-safe, memory safe, and can process  millions of events per second.

Tornado adheres to v2.0.0 of the [Semantic Versioning Initiative](http://semver.org/spec/v2.0.0.html).
It is fully open source.

<!-- The official repository is on [GitHub](link.html), and it is available under the X license. -->

## Contributing

<!-- Do we need to mention Support as some other projects do? -->

You can contribute to Tornado by reporting bugs, requesting features, or contributing code
on GitHub.  If you intend to submit a bug, please check first that someone else has not already
submitted it by searching the issue tracker on GitHub.

Check the ['contributing' documentation](CONTRIBUTING.md) for more details.

<!-- Do we have a forum or other feedback channel?  If so, should we mention it? -->


## Tornado crates documentation links

Tornado's crate docs are produced according to the
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


### Executables

Describes the structure of the Tornado binary executable, and the structure and configuration of many of its components.
- [tornado_engine](src/tornado/engine/doc/README.md)

The description of a binary executable that generates Tornado Events from rsyslog inputs.
- [tornado_rsyslog_collector](src/tornado/rsyslog_collector/doc/README.md)

A standalone HTTP server binary executable that listens for REST calls from a generic webhook.
- [tornado_webhook_collector](src/tornado/webhook_collector/doc/README.md)
