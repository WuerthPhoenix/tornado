.. _passive-monitoring:

Passive Monitoring
~~~~~~~~~~~~~~~~~~

NetEye's passive monitoring abilities count on Tornado, a Complex
Event Processor that receives reports of events from data sources such
as monitoring, email, and telegram, matches them against
pre-configured rules, and executes the actions associated with those
rules, which can include sending notifications, logging to files, and
annotating events in a time series graphing system.

Tornado is a high performance, scalable application, and is intended
to handle millions of events each second on standard server
hardware. Its overall architecture is depicted in
:numref:`figure-tornado-architecture`.

.. _figure-tornado-architecture:

.. figure:: /img/architecture.png
   :alt: Tornado architecture

   Tornado architecture

Tornado Architecture
````````````````````

Tornado is structured as a library, with three example binaries included
that show how it can be used. The three main components of the Tornado
architecture are:

* The *Tornado Collector(s)*, or just *Collector(s)*
* The *Tornado Engine*, or *Engine*
* The *Tornado Executor(s)*, or Executor(s)*

The term *Tornado* refers to the whole project or to a deployed system
that includes all three components.

Along with the main components, the following concepts are fundamental
to the Tornado architecture:

* A *Datasource*: A system that sends External Events* to Tornado, or
  a system to which Tornado subscribes to receive *External Events*.
* An *External Event*: An input received from a datasource. Its format
  depends on its source. An example of this is events from rsyslog.
* A *Tornado (or Internal) Event*: The Tornado-specific Event format.
* A *Rule*: A group of conditions that an Internal Event must match to
  trigger a set of Actions.
* An *Action*: An operation performed by Tornado, usually on an
  external system. For example, writing to Elastic Search or setting a
  state in a monitoring system.

Architecturally, Tornado is organized as a processing pipeline, where
input events move from collectors to the engine, to executors, without
branching or returning.

When the system receives an *External Event*, it first arrives at a
*Collector* where it is converted into a *Tornado Event*. Then it is
forwarded to the *Tornado Engine* where it is matched against
user-defined, composable *Rules*. Finally, generated *Actions* are
dispatched to the *Executors*.

The Tornado pipeline::

   Datasources (e.g. rsyslog)
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

Collectors
++++++++++

The purpose of a *Collector* is to receive and convert external events
into the internal Tornado Event structure, and forward them to the
Tornado Engine.

*Collectors* are *Datasource*-specific. For each datasource, there must
be at least one collector that knows how to manipulate the datasource’s
Events and generate Tornado Events.

Out of the box, Tornado provides a number of Collectors for handling
inputs from snmptrapd, rsyslog, JSON from Nats channels and generic
Webhooks.

Because all Collectors are defined with a simple format, Collectors for
new event types can easily be added or extended from existing types for:

* Monitoring events
* Email messages
* Telegram
* DNS
* Cloud monitoring (AWS, Azure, Cisco/Meraki, etc.)
* Netflow
* Elastic Stack
* SMS
* Operating system and authorization events

Engine
++++++

The *Engine* is the second step of the pipeline. It receives and
processes the events produced by the *Collectors*. The outcome of this
step is fully defined by a processing tree composed of *Filters* and
*Rule Sets*.

A *Filter* is a processing node that defines an access condition on the
children nodes.

A *Rule Set* is a node that contains an ordered set of *Rules*, where
each *Rule* determines:

* The conditions a *Tornado Event* has to respect to match it
* The actions to be executed in case of a match

The processing tree is parsed at startup from a configuration folder
where the node definitions are stored in JSON format.

When an event matches one or more *Rules*, the Engine produces a set of
*Actions* and forwards them to one or more *Executors*.

Executors
+++++++++

The *Executors* are the last element in the Tornado pipeline. They
receive the *Actions* produced from the *Engine* and trigger the
associated executable instructions.

An *Action* can be any command, process or operation. For example it can
include: \* Forwarding the events to a monitoring system \* Logging
events locally (e.g., as processed, discarded or matched) or remotely \*
Archiving events using software such as the Elastic Stack \* Invoking a
custom shell script

A single *Executor* usually takes care of a single *Action* type.

Tornado Crates Documentation Links
``````````````````````````````````

Tornado’s crate docs are produced according to the `Rust documentation
standards <https://doc.rust-lang.org/book/index.html>`__. The shortcuts
below, organized thematically, will take you to the documentation for
each module.

.. rubric:: Common Traits and Code

- :ref:`tornado-common-api` The Common API page describes the API and
  defines the Event and Action structures.

- :ref:`tornado-common-logger` The Logger page describes how Tornado
  logs its own actions.

.. _tornado-collectors:

.. rubric:: Collectors

- :ref:`tornado-common-collector` This crate describes the
  commonalities of all Collector types.

- :ref:`tornado-email-collector` Describes a collector that receives a
  MIME email message and generates an Event.

- :ref:`tornado-jmespath-collector` This page illustrates the Collector
  for JSON events using the JMESPath JSON query language.

- :ref:`tornado-json-collectors` Presents the standard JSON collector
  that deserializes an unstructured JSON string into an Event.

.. _tornado-engines:

.. rubric:: Engine

- :ref:`tornado-matcher-engine` The Matcher page describes the
  structure of the rules used in matching.

.. _tornado-executors:

.. rubric:: Executors

- :ref:`tornado-executor-common` This crate describes the
  commonalities of all Executor types.

- :ref:`tornado-archive-executor` This page describes how the Archive
  executor writes to log files on locally mounted file systems, with a
  focus on configuration.

- :ref:`tornado-icinga-executor` The Icinga2 executor forwards
  Tornado Actions to the `Icinga2 API
  <https://icinga.com/docs/icinga2/latest/12-icinga2-api>`__.

- :ref:`tornado-logger-executor` The Logger executor simply outputs
  the whole Action body to the standard `log
  <https://crates.io/crates/log>` at the *info* level.

- :ref:`tornado-script-executor` The Executor Script page defines how
  to configure Actions that launch shell scripts.

.. rubric:: Network

- :ref:`tornado-network-common` This page contains high level traits
  not bound to any specific network technology.

- :ref:`tornado-simple-network` Describes tests that dispatch Events
  and Actions on a single process without actually making network
  calls.

.. rubric:: Executables

- :ref:`tornado-engine-exec` Describes the structure of the Tornado
  binary executable, and the structure and configuration of many of
  its components.

- :ref:`tornado-email-collector-exec` An executable that processes
  incoming emails and generates Tornado Events.

- :ref:`tornado-icinga-collector-exec` An executable that subscribes
  to Icinga2 Event Streams API and generates Tornado Events.

- :ref:`tornado-nats-json-collector-exec` An executable that
  subscribes to Nats channels and generates Tornado Events.

- :ref:`tornado-rsyslog-collector-exec` The description of a binary
  executable that generates Tornado Events from *rsyslog* inputs.

- :ref:`tornado-snmptrap-collector` A Perl trap handler for Net-SNMP’s
  to subscribe to snmptrapd events.

- :ref:`tornado-webhook-collector-exec` A standalone HTTP server
  binary executable that listens for REST calls from a generic
  Webhook.

Tornado License
```````````````

Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
<LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option.
All files in the project carrying such notice may not be copied,
modified, or distributed except according to those terms.

.. include:: /61-tornado/collectors.rst
.. include:: /61-tornado/commonAPI.rst
.. include:: /61-tornado/engine.rst
.. include:: /61-tornado/engineAPI.rst
.. include:: /61-tornado/executor.rst
.. include:: /61-tornado/implementation.rst
.. include:: /61-tornado/network.rst
