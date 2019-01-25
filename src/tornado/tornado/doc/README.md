# Tornado

This crate contains the Tornado executable code.

The Tornado executable is a configuration of the matcher engine based on actix and 
built as a portable executable.

At runtime the executable opens two UDS sockets for receiving inputs from external collectors.

## Structure of Tornado

This specific Tornado executable is composed of the following components:
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
(e.g. Protobuf, Flatbuffer, CAP'n'proto) 
or with a message queue system (e.g. Nats.io, Kafka) creating a scalable distributed system.


### Structure and configuration: The json collector 
The [json collector](../../../collector/json/doc/README.md)
receives Events in JSON format and passes them to the matcher engine.

The events to be ingested by the JSON collector are published to the UDS socket
configured by the _uds-path_ command line parameter.

E.g.:
```bash
tornado --uds-path=/my/custom/path
```   

If not specified, Tornado will use the default value `/var/run/tornado/tornado.sock`.

### Structure and configuration: The snmptrapd collector
the [snmptrapd collector](../../../collector/snmptrapd/doc/README.md) receives snmptrap specific input, 
transform them in Tornado Events and forwards them to the matcher engine;

Snmptrapd are published to the UDS socket
configured by the _snmptrapd-uds-path_ command line parameter.

E.g.:
```bash
tornado --snmptrapd-uds-path=/my/custom/path
```   

If not specified, Tornado will use the default value `/var/run/tornado/tornado_snmptrapd.sock`.

The snmptrapd input documents should be in JSON format as described by the 
[collector's docs](../../../collector/snmptrapd/doc/README.md).


### Structure and configuration: The matching engine
The [matching engine](../../../engine/matcher/doc/README.md) receives Events from the collectors, 
processes them against the configured Rules and, in case of a match, produces the Actions to be 
performed.  

Two startup parameters determine the path to matcher configuration:
- _config-dir_: The filesystem folder where the Tornado configuration is saved; 
default value is __/etc/tornado__.
_ _rules-dir_: A folder relative to the _config_dir_ where the Rules are saved in JSON format; 
the default value is __/rules.d/__.

E.g.:
```bash
tornado --config-dir=/tornado/config --rules-dir=/rules
```  

Each Rule should be saved in the resulting configuration folder, 
in a dedicate file and in JSON format.

An example of a valid rule is:
```json
{
  "name": "emails_with_temperature",
  "description": "This matches all emails",
  "priority": 2,
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

This creates a rule with these properties:
- Its unique name is 'emails_with_temperature'. There cannot be two rules with the same name;
- Its priority is 2. The priority defines the execution order of the rules;
  '0' (zero) is the highest priority and denotes the first rule to be evaluated;
- An Event matches this Rule if it is of type "email", as requested by the _WHERE_ clause, and
  it is possible to extract the "temperature" variable from the "event.payload.body",
  as requested by the _WITH_ clause; 
- If an Event meets the Rule's requirements, the matcher produces an Action with _id_ "Logger"
  and the _payload_ with the three entries _type_, _subject_ and _temperature_. 

More information about the Rule's properties and the matcher can be found in the 
[matching engine documentation](../../../engine/matcher/doc/README.md) 


### Structure and configuration: The archive executor
The [archive executor](../../../executor/archive/doc/README.md) processes and executes Actions 
of type "archive".


### Structure and configuration: The script executor
The [script executor](../../../executor/script/doc/README.md) processes and executes Actions 
of type "script".

