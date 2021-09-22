.. _tornado-backend:

Tornado Backend
```````````````

This crate contains the Tornado backend code.

How It Works
++++++++++++

The Tornado backend contains endpoints that allow you to interact with
Tornado through
`REST <https://en.wikipedia.org/wiki/Representational_state_transfer>`__
endpoints. In the long run it will provide services to inspect, and let
you alter the configuration of Tornado and trigger custom events.

Tornado 'Auth' Backend API
++++++++++++++++++++++++++

The 'auth' APIs require the caller to pass an authorization token in the
headers in the format:

``Authorization : Bearer TOKEN_HERE``

The token should be a base64 encoded JSON with this user data:

.. code:: json

   {
     "user": "THE_USER_IDENTIFIER",
     "roles": ["ROLE_1", "ROLE_2", "ROLE_2"]
   }

In the coming releases the current token format will be replaced by a
`JSON Web Token (JWT) <https://en.wikipedia.org/wiki/JSON_Web_Token>`__.

.. rubric:: Auth endpoints

Endpoint: get list of draft ids

-  HTTP Method: **GET**
-  path : **/api/v1_beta/auth/who_am_i**
-  response type: **JSON**
-  response: a AuthWithPermissionDto with the current user profile
-  response example:
   
   .. code:: json

      {
         "user": "USERNAME",
         "permissions": ["ConfigEdit", "ConfigView"],
         "preferences": {
            "language": "en_US"
         }
      }

Tornado 'Config' Backend API
++++++++++++++++++++++++++++

The 'config' APIs require the caller to pass an authorization token in
the headers as in the 'auth' API.

.. rubric:: Working with configuration and drafts

These endpoints allow working with the configuration and the drafts

Endpoint: get the current Tornado configuration

-  HTTP Method: **GET**
-  path : **/api/v1_beta/config/current**
-  response type: **JSON**
-  response example:
   
   .. code:: json

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

Endpoint: get list of draft ids

-  HTTP Method: **GET**
-  path : **/api/v1_beta/config/drafts**
-  response type: **JSON**
-  response: An array of *String* ids
-  response example:
   
   .. code:: json

      ["id1", "id2"]

Endpoint: get a draft by id

-  HTTP Method: **GET**
-  path : **/api/v1_beta/config/drafts/{draft_id}**
-  response type: **JSON**
-  response: the draft content
-  response example:
   
   .. code:: json

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

Endpoint: create a new draft and return the draft id. The new draft is
an exact copy of the current configuration; anyway, a root Filter node
is added if not present.

-  HTTP Method: **POST**
-  path : **/api/v1_beta/config/drafts**
-  response type: **JSON**
-  response: the draft content
-  response example:
   
   .. code:: json

      {
        "id": "id3"
      }

Endpoint: update an existing draft

-  HTTP Method: **PUT**
-  path : **/api/v1_beta/config/drafts/{draft_id}**
-  request body type: **JSON**
-  request body: The draft content in the same JSON format returned by
   the **GET** **/api/v1_beta/config/drafts/{draft_id}** endpoint
-  response type: **JSON**
-  response: an empty json object

Endpoint: delete an existing draft

-  HTTP Method: **DELETE**
-  path : **/api/v1_beta/config/drafts/{draft_id}**
-  response type: **JSON**
-  response: an empty json object

Endpoint: take over an existing draft

-  HTTP Method: **POST**
-  path : **/api/v1_beta/config/drafts/{draft_id}/take_over**
-  response type: **JSON**
-  response: an empty json object

Endpoint: deploy an existing draft

-  HTTP Method: **POST**
-  path : **/api/v1_beta/config/drafts/{draft_id}/deploy**
-  response type: **JSON**
-  response: an empty json object

Tornado 'Event' Backend API
+++++++++++++++++++++++++++

.. rubric:: Send Test Event Endpoint

Endpoint: match an event on the current Tornado Engine configuration

-  HTTP Method: **POST**

-  path : **/api/v1_beta/event/current/send**

-  request type: **JSON**

-  request example:

   .. code:: json

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

   Where the event has the following structure:

   -  **type**: The Event type identifier
   -  **created_ms**: The Event creation timestamp in milliseconds since
      January 1, 1970 UTC
   -  **payload**: A Map<String, Value> with event-specific data
   -  **process_type**: Can be *Full* or *SkipActions*:

      -  *Full*: The event is processed and linked actions are executed
      -  *SkipActions*: The event is processed but actions are not
         executed

-  response type: **JSON**

-  response example:

   .. code:: json

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

Endpoint: match an event on a specific Tornado draft

-  HTTP Method: **POST**
-  path : **/api/v1_beta/event/drafts/{draft_id}/send**
-  request type: **JSON**
-  request/response example: same request and response of the
   **/api/v1_beta/event/current/send** endpoint

Tornado API DTOs
````````````````

The **tornado_api_dto** component contains the `Data Transfer
Object <https://en.wikipedia.org/wiki/Data_transfer_object>`__
definitions to carry data between processes.

These DTOs are the structures exposed by the REST endpoints of the
Tornado API.

The object structures are defined in the Rust programming language and
built as a Rust crate. In addition, at build time, in the *ts*
subfolder, `Typescript <https://www.typescriptlang.org/>`__ definitions
of the defined DTOs are generated.

These Typescript definitions can be imported by API clients written in
Typescript to provide compile-time type safety.

Generate the DTO Typescript definition files:
+++++++++++++++++++++++++++++++++++++++++++++

To generate the Typescript definitions files corresponding to the Rust
structures, execute the tests of this crate with the environment
variable **TORNADO_DTO_BUILD_REGENERATE_TS_FILES** set to *true*.

For example:

.. code:: bash

   TORNADO_DTO_BUILD_REGENERATE_TS_FILES=true cargo test 

The resulting *ts* will be generated in the **ts** subfolder.
