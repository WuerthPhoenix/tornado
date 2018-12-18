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
      |     |-- snmptrap
      |     |-- procmail
      |     |-- rsyslog
      |-- engine # The Tornado engine's core logic
      |     |-- parser # The parser is not necessary for the current architecture.
      |     |          # But it might be required by the correlation engine and/or for a custom DSL.
      |     |-- matcher # The matcher contains the logic that evaluates whether an Event matches a rule,
      |     |           # and to trigger the related Actions.
      |-- common # Common interfaces and message definitions
      |     |-- api    # Global traits required by the engine, collectors and executors
      |     |-- logger # The logger configuration
      |-- network # An abstract service which will be used by all components to communicate with each other
      |     |-- common
      |     |-- simple
      |     |-- nats
      |     |-- nanomsg
      |-- RestApi
      |     |-- http



# How to Reference Fields

    event.<variablename>
    event.type
    event.created_ts
     
    event.payload.<variablename>
    event.payload.Subject
    event.payload.Body
     
    matched.<rulename>.<variablename>
    matched.tempmail.Extracted_Temp
     
    _variables.<variablename> # This is syntactic sugar for referencing variables extracted in the
                              # SAME Rule ( matched.<own_rule_name>.<variablename> )



# Data Structures (No Guarantee for Completeness)



## Incoming Event Structure (Collector Output)

    {
        "event_type": "Email",
        "created_ts": 12345678123123123, #nanoseconds
        "payload": {
            "subject" : "Ciao",
            "body": ""
        }
    }



## Internal Representation (During Processing)

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
    # - The process outcome status
    # - The list of actions generated
    # - An optional text message.  This is used, for example, to report eventual processing errors
    #   or for debugging purposes.
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
              "message": "Could not resolve the extracted variable [variable_name]"
          }
       },
     
    # Contains a list of the extracted variables, prefixed by the rule name, and their computed values
       "extracted_vars": {
           "syslog_rule.extracted_temp": "42 Degree",
           "syslog_rule.body": "body content",
           "emailrule123.extracted_temp": "42 Degree"
       }
    }



# Definition of a Rule

    {

       "name": "all_emails_and_syslogs", # Only the characters that matches the regex [a-z0-9_]+ are valid.
       "description": "This is All Emails and Syslogs", # A human-readable description. We could prepare
                                                        # a suggested name for the frontend based on this.
       "priority": 42,
       "continue": true, # Continue processing after matching this rule.
       "active": true,
       "constraint": {
            # Constraint evaluation happens in sequence, first WHERE, then WITH, for performance reasons.
            # Query optimization would be nice to have for future versions.  For now, the user is responsible for query performance.
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
                       # Syntax + Escaping to be defined:  Current proposal behavior of bash limited to
                       # ${VAR_NAME} notation, without recursive resolving
                       "command": "/usr/bin/sudo     /usr/bin/rm -rf '$    {_variables.extracted_temp}p'     --no-preserve-root \${HOME}p",
                   }
               }
           }
       ]
    }



# Internal Representation of an Action Call

    {
        "id": "Monitoring",
        "payload" : {
            "host": "neteye.local",
            "service": "PING",
            "state": "CRITICAL",
            "comment": "42 Degrees"
        }
    }
