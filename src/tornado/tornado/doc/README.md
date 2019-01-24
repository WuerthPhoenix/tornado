# Tornado

This crate contains the Tornado executable code.

The Tornado executable is a configuration the engine based on actix and 
built as a portable executable.

At runtime the executable opens two UDS sockets for receiving inputs from external collectors.

## Structure of Tornado

This Tornado executable is composed of the following components:
- the json collector 
- the snmptrapd collector
- the matching engine 
- the archive executor
- the script executor
    
Each component is wrapped into a dedicated actix actor.
 
This configuration is only one of the many possible. 
Each component is, in fact, developed as an independent library allowing 
great flexibility in deciding whether and how to use it.

At the same, there are no restrictions that force the use of the components into the same 
executable.
While this is the simplest way of assembling them into a working product, the collectors 
and executors could live on their own executables and communicate with the Tornado engine 
through remote call. 
This can be achieved through direct TCP or HTTP call, using an RPC technology 
(e.g. ToDo add links, Protobuf, Flatbuffer, CAP'n'proto) 
or with a message queue system (e.g. Nats.io, Kafka) creating a scalable distributed system.


### Structure and configuration: The json collector 
The json collector (ToDo add link to doc)  
receives Events in JSON format and passes them to the matcher engine.

The events to be ingested by the JSON collector are published to the UDS socket
configured by the _uds-path_ command line parameter.

E.g.:
```bash
tornado --uds-path=/my/custom/path
```   

If not specified, Tornado will use the default value `/var/run/tornado/tornado.sock`.

### Structure and configuration: The snmptrapd collector
the snmptrapd collector (ToDo add link to doc) receives snmptrap specific input, transform them in 
Tornado Events and forwards them to the matcher engine;

Snmptrapd are published to the UDS socket
configured by the _snmptrapd-uds-path_ command line parameter.

E.g.:
```bash
tornado --snmptrapd-uds-path=/my/custom/path
```   

If not specified, Tornado will use the default value `/var/run/tornado/tornado_snmptrapd.sock`.

The snmptrapd input documents should be in JSON format as described by the collector's docs.


### Structure and configuration: The matching engine
The matching engine (ToDo add link to doc) receives Events from the collectors, processes 
them against the configured Rules and, in case of a match, produces the Actions to be performed;  

### Structure and configuration: The archive executor
The archive executor (ToDo add link to doc) processes and executes Actions of type "archive".

### Structure and configuration: The script executor
The script executor (ToDo add link to doc) processes and executes Actions of type "script".

