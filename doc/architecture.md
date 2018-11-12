# Target Project Structure

    Tornado
      |-- executor
      |     |-- common
      |     |-- logger
      |     |-- archive_influx
      |     |-- archive_filesystem
      |     |-- log_es
      |     |-- Monitoring
      |-- collector
      |     |-- common
      |     |-- json
      |     |-- icinga2
      |     |-- snmtptrap
      |     |-- procmail
      |     |-- rsyslog
      |-- engine # Tornado engine core logic
      |     |-- parser # the parser is non needed with the currently architecture.
      |     |          # It could anyway be required by the correlation engine and/or for a custom DSL
      |     |-- matcher # the matcher contains the logic to evaluate whether an Event matches a rule and to trigger the related Actions.
      |-- common #Common interfaces and message definitions
      |     |-- api    # global traits required by engine, collectors and executors
      |     |-- logger # logger configuration
      |-- network #Abstract service which will be used by all components to communicate
      |     |-- common
      |     |-- simple
      |     |-- nats
      |     |-- nanomsg
      |-- RestApi
      |     |-- http

# How to reference to fields

    event.<variablename>
    event.type
    event.created_ts
     
    event.payload.<variablename>
    event.payload.Subject
    event.payload.Body
     
    matched.<rulename>.<variablename>
    matched.tempmail.Extracted_Temp
     
    _variables.<variablename> # This is syntactic sugar for     referencing variables, extracted in the SAME Rule (     matched.<own_rule_name>.<variablename> )

# Datastructures (no guarantee for completeness)

## Incoming Event (collector output)

    {
        "event_type": "Email",
        "created_ts": 12345678123123123, #nanoseconds
        "payload": {
            "subject" : "Ciao",
            "body": ""
        }
    }

## Internal Representation (during processing)

    {
       "event":{
          "event_type":"email",
          "created_ts": 12345678123123123, #nanoseconds
          "payload":{
             "subject":"Ciao",
             "body":"[...] 42 Degrees, 51% Humidity[...]",
          }
       },
    #
    # Processed Rules are store here.
    # Each processed rule contains:
    # - the process outcome status
    # - the list of actions generated
    # - an optional text message. This is used, for example, to report eventual processing errors or for debugging purpose.
    #
       "rules":{
          "syslog_rule":{
             "status": "Matched",
             "actions:": [
                 {
                  "id": "log",
                  "payload": {}
                 }
             ]
          },
          "emailrule123":{
              "status": "PartiallyMatched",
              "actions:": [
                  {
                      "id": "log",
                      "payload": {}
                  }
              ]
          },
          "all_emails_and_syslogs":{
              "status": "NotMatched",
              "message": "Could not resolve the extracted     variable [variable_name]"
          }
       },
     
    # Contains the extracted variables, prefixed by the rule     name, and their computed value
       "extracted_vars": {
           "syslog_rule.extracted_temp": "42 Degree",
           "syslog_rule.body": "body content",
           "emailrule123.extracted_temp": "42 Degree"
       }
    }

#Definition of a Rule

    {

       "name": "all_emails_and_syslogs", # Validated only     [a-z0-9_]+
       "description" : "This is All Emails and Syslogs", #     Human readable description. We could prepare a     suggested name in the frontend based on this.
       "priority": 42,
       "continue": true,  # Continue processing after     matching this rule
       "active": true,
       "constraint": {   
            # Constraint evaluation happens in sequence,     first WHERE, then WITH for performance reasons
            # Query optimization is a nice to have for     future versions. For now the user is     responsible for query performance
            "WHERE": {
                "type": "AND",
                "operators": [
                    {
                        "type": "equal",
                        "first": "${event.type}",
                        "second": "email"
                    },
                    {
                        "type": "OR",
                        "operators": [
                            {
                                "type": "equal",
                                "first": "$    {event.payload.body}",
                                "second": "something"
                            },
                            {
                                "type": "equal",
                                "first": "$    {event.payload.body}",
                                "second": "other"
                            }
                        ]
                    }
                ]
            },
            "WITH": {
                "extracted_temp": {
                   "from":"${event.payload.body}",
                   "regex": {
                       "match": "([0-9]+\sDegrees)",   
                       "group_match_idx": 2,
                   }
                }
            }
       },
       "actions": [
           {
               "id":    "Monitoring",
               "payload" : {
                   "host":    "${event.payload.hostname}",
                   "service": "${event.payload.service}",
                   "state":   "CRITICAL"
                   "comment": "${_variables.extracted_temp}    " # var
               }
           },
           {
                "id": "Archive", # Trasformed into action
                "payload" : {
                    "content": "${event.payload.subject}$    {event.payload.body}"
                }
           {
               "id": "Command",
               "payload" : {
                   {
                       # Syntax + Escaping to be defined:     Current proposal behaviour of bash     limited to ${VAR_NAME} notation,     without recursive resolving
                       "command": "/usr/bin/sudo     /usr/bin/rm -rf '$    {_variables.extracted_temp}p'     --no-preserve-root \${HOME}p",
                   }
               }
           }
       ]
    }


# Internal Representation of Action Call

    {
        "id": "Monitoring",
        "payload" : {
            "host": "neteye.local",
            "service": "PING",
            "state": "CRITICAL",
            "comment": "42 Degrees"
        }
    }
