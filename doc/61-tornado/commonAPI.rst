.. _tornado-common-api:

Common API
``````````

The *tornado_common_api* crate contains the API for cross-component
communication. Three main components are currently defined:

-  The Collectors
-  The Tornado Engine
-  The Executors

The Tornado Engine receives Events from the Collectors, checks them
against a set of Rules, and then triggers Actions defined within the
matched Rule(s) by sending messages to the appropriate executors.

Events
``````

An Event has a simple structure, composed as follows:

-  **type**: The Event type identifier (a given Collector usually sends
   Events of only a single type)
-  **created_ms**: The Event creation timestamp in milliseconds since
   January 1, 1970 UTC
-  **payload**: A Map<String, Value> with event-specific data

where the payload **Value** can be any valid JSON type:

-  A **null** value
-  A **string**
-  A **bool** value (i.e., true or false)
-  A **number**
-  An **array** of Values
-  A **map** of type Map<String, Value>

All fields must have values, although the *payload* can be an empty
structure.

Example Event in JSON format:

.. code:: json

   {
       "type": "email",
       "created_ms": 1554130814854,
       "payload": {
           "subject" : "Doing something",
           "body": "everything's done"
       }
   }

Actions
```````

Like the Event structure, the structure of an Action is simple:

-  **id**: The Action type identifier (a given Executor usually
   processes just a single Action type)
-  **payload**: A Map<String, Value> with data specific to its Action.

All fields must have values, although again the *payload* can be an
empty structure.

Example Action in JSON format:

.. code:: json

   {
       "id": "Monitoring",
       "payload" : {
           "host": "neteye.local",
           "service": "PING",
           "state": "CRITICAL",
           "comment": "42 Degrees"
       }
   }
