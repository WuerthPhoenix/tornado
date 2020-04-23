# Tornado Backend

This crate contains the Tornado backend code.


## How It Works

The Tornado backend contains endpoints that allow you to interact with Tornado through
[REST](https://en.wikipedia.org/wiki/Representational_state_transfer) endpoints.
In the long run it will provide services to inspect, and let you alter the configuration
of Tornado and trigger custom events.

## Tornado 'Config' Backend API

The 'config' APIs require the caller to pass an authorization token in the headers in the format:

`Authorization : Bearer TOKEN_HERE`

The token should be a base64 encoded JSON with this user data:
```json
{
  "user": "THE_USER_IDENTIFIER",
  "roles": ["ROLE_1", "ROLE_2", "ROLE_2"]
}
```  

In the coming releases the current token format will be replaced by a 
[JSON Web Token (JWT)](https://en.wikipedia.org/wiki/JSON_Web_Token).
 

### Working with configuration drafts
These endpoints allow the editing of configuration drafts


Endpoint: get list of draft ids
- HTTP Method: __GET__
- path : __/api/v1/config/drafts__
- response type: __JSON__
- response: An array of _String_ ids  
- response example:
  ```json
  ["id1", "id2"]
  ```
  
  
Endpoint: get a draft by id
- HTTP Method: __GET__
- path : __/api/v1/config/draft/{draft_id}__
- response type: __JSON__
- response: the draft content
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
           "WHERE": {},
           "WITH": {}
         },
         "actions": []
       }
     ]
   }
  ```


Endpoint: create a new and return the draft id
- HTTP Method: __POST__
- path : __/api/v1/config/draft__
- response type: __JSON__
- response: the draft content
- response example:
  ```json
  {
    "id": "id3"
  }
  ```
  
  
Endpoint: update an existing draft
- HTTP Method: __PUT__
- path : __/api/v1/config/draft/{draft_id}__
- request body type: __JSON__
- request body: The draft content in the same JSON format returned by the 
  __GET__ __/api/v1/config/draft/{draft_id}__ endpoint 
- response type: __JSON__
- response: an empty json object


Endpoint: delete an existing draft
- HTTP Method: __DELETE__
- path : __/api/v1/config/draft/{draft_id}__
- response type: __JSON__
- response: an empty json object
  

## Tornado 'Event' Backend API

### Get Configuration Endpoint 

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


### Send Test Event Endpoint 

This endpoint receives an _Event_, processes it, and returns the result of the Tornado Engine 
processing that event.

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

  Where the event has the following structure:
  - __type__:  The Event type identifier
  - __created_ms__:  The Event creation timestamp in milliseconds since January 1, 1970 UTC
  - __payload__:  A Map<String, Value> with event-specific data
  - __process_type__:  Can be _Full_ or _SkipActions_:
    - _Full_:  The event is processed and linked actions are executed
    - _SkipActions_:  The event is processed but actions are not executed
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
