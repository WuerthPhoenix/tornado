# Tornado backend

This crate contains the Tornado backend code.


## How It Works

The Tornado backend contains endpoints that allow you to interact with Tornado.
In the long run, it will provide services to inspect and alter the configuration
of Tornado and to trigger custom events.

## Tornado backend API

### Get configuration endpoint 

This endpoint returns the current Tornado configuration.

Details:
- HTTP Method: __GET__
- path : __/api/config__
- response type: __JSON__ 
- response example:
   ```json
   {
     "type": "Rules",
     "rules": [
       {
         "name": "all_emails",
         "description": "This matches all emails",
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
           "WITH": {}
         },
         "actions": [
           {
             "id": "Logger",
             "payload": {
               "subject": "${event.payload.subject}",
               "type": "${event.type}"
             }
           }
         ]
       }
     ]
   }
   ```

### Send Test Event endpoint 

This endpoint receives an _Event_, processes it and returns the Tornado Engine 
process outcome.

Details:
- HTTP Method: __POST__
- path : __/api/send_event__
- request type: __JSON__
- request example:
  ```json
  {
      "event": {
        "type": "the_event_type",
        "created_ms": 123456,
        "payload": {
          "value_one": "something",
          "value_two": "something_else"
        }
      },
      "process_type": "SkipActions"
  }
  ```
  Where:
  - __type__:  The Event type identifier
  - __created_ms__:  The Event creation timestamp in milliseconds since January 1, 1970 UTC
  - __payload__:  A Map<String, Value> with event-specific data
  - __process_type__: Can be _Full_ or _SkipActions_:
    - _Full_: the event is processed and linked actions are executed.
    - _SkipActions_: the event is processed but actions are not executed.
- response type: __JSON__ 
- response example:
   ```json
  {
    "event": {
      "type": "the_event_type",
      "created_ms": 123456,
      "payload": {
        "value_one": "something",
        "value_two": "something_else"
      }
    },
    "result": {
      "type": "Rules",
      "rules": {
        "rules": {
          "emails_with_temperature": {
            "rule_name": "emails",
            "status": "NotMatched",
            "actions": [],
            "message": null
          },
          "archive_all": {
            "rule_name": "archive_all",
            "status": "Matched",
            "actions": [
              {
                "id": "archive",
                "payload": {
                  "archive_type": "one",
                  "event": {
                    "created_ms": 123456,
                    "payload": {
                      "value_one": "something",
                      "value_two": "something_else"
                    },
                    "type": "the_event_type"
                  }
                }
              }
            ],
            "message": null
          }
        },
        "extracted_vars": {}
      }
    }
  }
   ```
