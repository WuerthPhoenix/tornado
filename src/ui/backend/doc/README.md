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
