Executor Common
```````````````

The *tornado_executor_common* crate contains the Trait definitions for
the Executors.

An Executor is in charge of performing a specific Action (usually only
one, but sometimes more). It receives an action description from the
Tornado engine and delivers the operation linked to it.

.. _tornado-director-executor:

Director Executor
`````````````````

The Director Executor is an application that extracts data from a
Tornado Action and prepares it to be sent to the `Icinga Director REST
API <https://icinga.com/docs/director/latest/doc/70-REST-API/>`__.

How It Works
++++++++++++

This executor expects a Tornado Action to include the following elements
in its payload:

1. An **action_name**: The Director action to perform.
2. An **action_payload** (optional): The payload of the Director action.
3. An **icinga2_live_creation** (optional): Boolean value, which
   determines whether to create the specified Icinga Object also in
   Icinga2.

Valid values for **action_name** are:

-  **create_host**: creates an object of type ``host`` in the Director
-  **create_service**: creates an object of type ``service`` in the
   Director

The **action_payload** should contain at least all mandatory parameters
expected by the Icinga Director REST API for the type of object you want
to create.

An example of a valid Tornado Action is:

.. code:: json

   {
       "id": "director",
       "payload": {
           "action_name": "create_host",
           "action_payload": {
             "object_type": "object",
             "object_name": "my_host_name",
             "address": "127.0.0.1",
             "check_command": "hostalive",
             "vars": {
               "location": "Bolzano"
             }
           },
           "icinga2_live_creation": true
       }
   }
   
.. _tornado-logger-executor:

Logger Executor
```````````````

An Executor that logs received Actions.

How it Works
++++++++++++

The Logger executor simply outputs the whole Action body to the standard
`log <https://crates.io/crates/log>`__ at the *info* level.

.. _tornado-script-executor:

Script Executor
```````````````

An executor that runs custom shell scripts on a Unix-like system.

How It Works
++++++++++++

To be correctly processed by this executor, an Action should provide two
entries in its payload: the path to a script on the local filesystem of
the executor process, and all the arguments to be passed to the script
itself.

The script path is identified by the payload key **script**. It is
important to verify that the executor has both read and execute rights
at that path.

The script arguments are identified by the payload key **args**; if
present, they are passed as command line arguments when the script is
executed.

An example of a valid Action is:

.. code:: json

   {
       "id": "script",
       "payload" : {
           "script": "./usr/script/my_script.sh",
           "args": [
               "tornado",
               "rust"
           ] 
       }
   }

In this case the executor will launch the script *my_script.sh* with the
arguments "tornado" and "rust". Consequently, the resulting command will
be:

.. code:: bash

   ./usr/script/my_script.sh tornado rust

Other Ways of Passing Arguments
+++++++++++++++++++++++++++++++

There are different ways to pass the arguments for a script:

-  Passing arguments as a String:

   .. code:: json

      {
        "id": "script",
        "payload" : {
            "script": "./usr/script/my_script.sh",
            "args": "arg_one arg_two -a --something else"
        }
      }

   If **args** is a String, the entire String is appended as a single
   argument to the script. In this case the resulting command will be:

   .. code:: bash

      ./usr/script/my_script.sh "arg_one arg_two -a --something else" 

-  Passing arguments in an array:

   .. code:: json

      {
        "id": "script",
        "payload" : {
            "script": "./usr/script/my_script.sh",
            "args": [
                "--arg_one tornado",
                "arg_two",
                true,
                100
            ] 
        }
      }

   Here the argument's array elements are passed as four arguments to
   the script in the exact order they are declared. In this case the
   resulting command will be:

   .. code:: bash

      ./usr/script/my_script.sh "--arg_one tornado" arg_two true 100 

-  Passing arguments in a map:

   .. code:: json

      {
        "id": "script",
        "payload" : {
            "script": "./usr/script/my_script.sh",
            "args": {
              "arg_one": "tornado",
              "arg_two": "rust"
          }
        }
      }

   When arguments are passed in a map, each entry in the map is
   considered to be a (option key, option value) pair. Each pair is
   passed to the script using the default style to pass options to a
   Unix executable which is *--key* followed by the *value*.
   Consequently, the resulting command will be:

   .. code:: bash

      ./usr/script/my_script.sh --arg_one tornado --arg_two rust

   Please note that ordering is not guaranteed to be preserved in this
   case, so the resulting command line could also be:

   .. code:: bash

      ./usr/script/my_script.sh --arg_two rust --arg_one tornado

   Thus if the order of the arguments matters, you should pass them
   using either the string- or the array-based approach.

-  Passing no arguments:

   .. code:: json

      {
        "id": "script",
        "payload" : {
            "script": "./usr/script/my_script.sh"
        }
      }

   Since arguments are not mandatory, they can be omitted. In this case
   the resulting command will simply be:

   .. code:: bash

      ./usr/script/my_script.sh

.. _tornado-monitoring-executor:

Monitoring Executor
```````````````````

The Monitoring Executor is an executor that permits to perform Icinga
``process check results`` also in the case that the Icinga object for
which you want to perform the ``process check result`` does not yet
exist.

This is done by means of executing the action ``process check result``
with the Icinga Executor, and by executing the actions
``create_host``/``create_service`` with the Director Executor, in case
the underlying Icinga objects do not yet exist in Icinga.

.. warning:: The Monitoring Executor requires the live-creation
   feature of the Icinga Director to be exposed in the REST API. If
   this is not the case, the actions of this executor will always fail
   in case the Icinga Objects are not already present in Icinga2.

How It Works
++++++++++++

This executor expects a Tornado Action to include the following elements
in its payload:

1. An **action_name**: The Monitoring action to perform.
2. A **process_check_result_payload**: The payload for the Icinga2
   ``process check result`` action.
3. A **host_creation_payload**: The payload which will be sent to the
   Icinga Director REST API for the host creation.
4. A **service_creation_payload**: The payload which will be sent to the
   Icinga Director REST API for the service creation (mandatory only in
   case **action_name** is
   ``create_and_or_process_service_passive_check_result``).

Valid values for **action_name** are:

-  **create_and_or_process_host_passive_check_result**: sets the
   ``passive check result`` for a ``host``, and, if necessary, it also
   creates the host.
-  **create_and_or_process_service_passive_check_result**: sets the
   ``passive check result`` for a ``service``, and, if necessary, it
   also creates the service.

The **process_check_result_payload** should contain at least all
mandatory parameters expected by the Icinga API to perform the action.
The object on which you want to set the ``passive check result`` must be
specified with the field ``host`` in case of action
**create_and_or_process_host_passive_check_result**, and ``service`` in
case of action **create_and_or_process_service_passive_check_result**
(e.g. specifying a set of objects on which to apply the
``passive check result`` with the parameter ``filter`` is not valid)

The **host_creation_payload** should contain at least all mandatory
parameters expected by the Icinga Director REST API to perform the
creation of a host.

The **servie_creation_payload** should contain at least all mandatory
parameters expected by the Icinga Director REST API to perform the
creation of a service.

An example of a valid Tornado Action is:

.. code:: json

   {
     "id": "monitoring",
     "payload": {
       "action_name": "create_and_or_process_service_passive_check_result",
       "process_check_result_payload": {
         "exit_status": "2",
         "plugin_output": "Output message",
         "service": "myhost!myservice",
         "type": "Service"
       },
       "host_creation_payload": {
         "object_type": "Object",
         "object_name": "myhost",
         "address": "127.0.0.1",
         "check_command": "hostalive",
         "vars": {
           "location": "Rome"
         }
       },
       "service_creation_payload": {
         "object_type": "Object",
         "host": "myhost",
         "object_name": "myservice",
         "check_command": "ping"
       }
     }
   }

The flowchart shown in :numref:`fig-monitoring-executor-flowchart`
helps to understand the behaviour of the Monitoring Executor in
relation to Icinga2 and Icinga Director REST APIs.

.. _fig-monitoring-executor-flowchart:

.. figure:: /img/monitoring-executor-flowchart.png

   Flowchart of Monitoring Executor.

.. _tornado-foreach-executor:

Foreach Executor
````````````````

An Executor that loops through a set of data and executes a list of
actions for each entry.

How it Works
++++++++++++

The Foreach executor extracts all values from an array of elements and
injects each value to a list of action under the *item* key.

It has two mandatory configuration entries in its payload:

-  **target**: the array of elements
-  **actions**: the array of action to execute

For example, given this rule definition:

.. code:: json

   {
     "name": "do_something_foreach_value",
     "description": "This uses a foreach loop",
     "continue": true,
     "active": true,
     "constraint": {
       "WITH": {}
     },
     "actions": [
       {
         "id": "foreach",
         "payload": {
           "target": "${event.payload.values}",
           "actions": [
             {
               "id": "logger",
               "payload": {
                 "source": "${event.payload.source}",
                 "value": "the value is ${item}"
               }
             },
             {
               "id": "archive",
               "payload": {
                 "event": "${event}",
                 "item_value": "${item}"
               }
             }
           ]
         }
       }
     ]
   }

When an event with this payload is received:

.. code:: json

   {
     "type": "some_event",
     "created_ms": 123456,
     "payload":{
       "values": ["ONE", "TWO", "THREE"],
       "source": "host_01"
     }
   }

Then the **target** of the foreach action is the array
``["ONE", "TWO", "THREE"]``; consequently, each one of the two inner
actions is executed three times; the first time with *item* = "ONE",
then with *item* = "TWO" and, finally, with *item* = "THREE".


.. _tornado-archive-executor:

Archive Executor
````````````````

The Archive Executor is an executor that writes the Events from the
received Actions to a file.

Requirements and Limitations
++++++++++++++++++++++++++++

The archive executor can only write to locally mounted file systems. In
addition, it needs read and write permissions on the folders and files
specified in its configuration.

Configuration
+++++++++++++

The archive executor has the following configuration options:

-  **file_cache_size**: The number of file descriptors to be cached. You
   can improve overall performance by keeping files from being
   continuously opened and closed at each write.
-  **file_cache_ttl_secs**: The *Time To Live* of a file descriptor.
   When this time reaches 0, the descriptor will be removed from the
   cache.
-  **base_path**: A directory on the file system where all logs are
   written. Based on their type, rule Actions received from the Matcher
   can be logged in subdirectories of the base_path. However, the
   archive executor will only allow files to be written inside this
   folder.
-  **default_path**: A default path where all Actions that do not
   specify an ``archive_type`` in the payload are logged.
-  **paths**: A set of mappings from an archive_type to an
   ``archive_path``, which is a subpath relative to the base_path. The
   archive_path can contain variables, specified by the syntax
   ``${parameter_name}``, which are replaced at runtime by the values in
   the Action's payload.

The archive path serves to decouple the type from the actual subpath,
allowing you to write Action rules without worrying about having to
modify them if you later change the directory structure or destination
paths.

As an example of how an archive_path is computed, suppose we have the
following configuration:

.. code:: toml

   base_path =  "/tmp"
   default_path = "/default/out.log"
   file_cache_size = 10
   file_cache_ttl_secs = 1

   [paths]
   "type_one" = "/dir_one/file.log"
   "type_two" = "/dir_two/${hostname}/file.log"

and these three incoming actions:

*action_one*:

.. code:: json

   {
       "id": "archive",
       "payload": {
           "archive_type": "type_one",
           "event": "__the_incoming_event__"
       }
   }

*action_two*:

.. code:: json

   {
       "id": "archive",
       "payload": {
           "archive_type": "type_two",
           "hostname": "net-test",
           "event": "__the_incoming_event__"
       }
   }

*action_three*:

.. code:: json

   {
       "id": "archive",
       "payload": {
           "event": "__the_incoming_event__"
       }
   }

then:

-  *action_one* will be archived in **/tmp/dir_one/file.log**
-  *action_two* will be archived in **/tmp/dir_two/net-test/file.log**
-  *action_three* will be archived in **/tmp/default/out.log**

How it Works
++++++++++++

The archive executor expects an Action to include the following elements
in the payload:

1. An **event**: The Event to be archived should be included in the
   payload under the key ``event``.
2. An **archive type** (optional): The archive type is specified in the
   payload under the key ``archive_type``.

When an archive_type is not specified, the default_path is used (as in
action_three). Otherwise, the executor will use the archive_path in the
``paths`` configuration corresponding to the ``archive_type`` key
(action_one and action_two).

When an archive_type is specified but there is no corresponding key in
the mappings under the ``paths`` configuration, or it is not possible to
resolve all path parameters, then the Event will not be archived.
Instead, the archiver will return an error.

The Event from the payload is written into the log file in JSON format,
one event per line.

.. _tornado-elasticsearch-executor:

Elasticsearch Executor
``````````````````````

The Elasticsearch Executor is a functionality that extracts data from a
Tornado Action and sends it to
`Elasticsearch <https://www.elastic.co/guide/en/elasticsearch/reference/current/rest-apis.html>`__.

How It Works
++++++++++++

The executor expects a Tornado Action that includes the following
elements in its payload:

1. An **endpoint** : The Elasticsearch endpoint which Tornado will call
   to create the Elasticsearch document.
2. An **index** : The name of the Elasticsearch index in which the
   document will be created.
3. An **data**: The content of the document that will be sent to
   Elasticsearch.
4. (**optional**) An **auth**: a method of authentication, see next
   section.

An example of a valid Tornado Action is a json document like this:

.. code:: json

   {
       "id": "elasticsearch",
       "payload": {
           "endpoint": "http://localhost:9200",
           "index": "tornado-example",
           "data": {
               "user" : "kimchy",
               "post_date" : "2009-11-15T14:12:12",
               "message" : "trying out Elasticsearch"
           }
       }
   }

The Elasticsearch Executor will create a new document in the specified
Elasticsearch index for each action executed; also the specified index
will be created if it does not already exist.

In the above json document, no authentication is specified, therefore
the default authentication method created during the executor creation
is used. This method is saved in a tornado configuration file
(``elasticsearch_executor.toml``) and can be overridden for each Tornado
Action, as described in the next section.

Elasticsearch authentication
++++++++++++++++++++++++++++

When the Elasticsearch executor is created, a default authentication
method can be specified and will be used to authenticate to
Elasticsearch, if not differently specified by the action. On the
contrary, if a default method is **not** defined at creation time, then
each action that does not specify an authentication method **will
fail**.

To use a specific authentication method the action should include the
``auth`` field with either of the following authentication types:
**None** or **PemCertificatePath**, like shown in the following
examples.

-  **None**: the client connects to Elasticsearch without authentication

   Example:

   .. code:: json

      {
          "id": "elasticsearch",
          "payload": {
              "index": "tornado-example",
              "endpoint": "http://localhost:9200",
              "data": {
                  "user": "myuser"
              },
              "auth": {
                  "type": "None"
              }
          }
      }

-  **PemCertificatePath**: the client connects to Elasticsearch using
   the PEM certificates read from the local file system. When this
   method is used, the following information must be provided:

   -  **certificate_path**: path to the public certificate accepted by
      Elasticsearch
   -  **private_key_path**: path to the corresponding private key
   -  **ca_certificate_path**: path to CA certificate needed to verify
      the identity of the Elasticsearch server

   Example:

   .. code:: json

      {
          "id": "elasticsearch",
          "payload": {
              "index": "tornado-example",
              "endpoint": "http://localhost:9200",
              "data": {
                  "user": "myuser"
              },
              "auth": {
                  "type": "PemCertificatePath",
                  "certificate_path": "/path/to/tornado/conf/certs/tornado.crt.pem",
                  "private_key_path": "/path/to/tornado/conf/certs/private/tornado.key.pem",
                  "ca_certificate_path": "/path/to/tornado/conf/certs/root-ca.crt"
              }
          }
      }

.. _tornado-icinga-executor:

Icinga2 Executor
````````````````

The Icinga2 Executor is an executor that extracts data from a Tornado
Action and prepares it to be sent to the `Icinga2
API <https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api>`__.

How It Works
++++++++++++

This executor expects a Tornado Action to include the following elements
in its payload:

1. An **icinga2_action_name**: The Icinga2 action to perform.
2. An **icinga2_action_payload** (optional): The parameters of the
   Icinga2 action.

The **icinga2_action_name** should match one of the existing `Icinga2
actions <https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#actions>`__.

The **icinga2_action_payload** should contain at least all mandatory
parameters expected by the specific Icinga2 action.

An example of a valid Tornado Action is:

.. code:: json

   {
       "id": "icinga2",
       "payload": {
           "icinga2_action_name": "process-check-result",
           "icinga2_action_payload": {
               "exit_status": "${event.payload.exit_status}",
               "plugin_output": "${event.payload.plugin_output}",
               "filter": "host.name==\"example.localdomain\"",
               "type": "Host"
           }
       }
   }

.. _tornado-smartmon-check-executor:

Smart Monitoring Check Result Executor
``````````````````````````````````````

The Smart Monitoring Check Result Executor permits to perform an Icinga
``process check results`` also in case the Icinga object for which you
want to carry out that action does not exist.

This is done by first running the action ``process check result`` with
the Icinga Executor, and then, in case the underlying Icinga objects do
not exist in Icinga, the actions ``create_host``/``create_service`` with
the Director Executor.

.. warning:: The Smart Monitoring Check Result Executor requires the
   live-creation feature of the Icinga Director to be exposed in the
   REST API. If this is not the case, the actions of this executor
   will always fail in case the Icinga Objects are not already present
   in Icinga2.

How It Works
++++++++++++

This executor expects a Tornado Action to include the following elements
in its payload:

1. A **check_result**: The basic data to build the Icinga2
   ``process check result`` action payload.
2. A **host**: The data to build the payload which will be sent to the
   Icinga Director REST API for the host creation.
3. A **service**: The data to build the payload which will be sent to
   the Icinga Director REST API for the service creation (optional).

The **check_result** should contain all mandatory parameters expected by
the Icinga API except the following ones that are automatically filled
by the executor:

-  ``host``
-  ``service``
-  ``type``

The **host** and **service** should contain all mandatory parameters
expected by the Icinga Director REST API to perform the creation of a
host and/or a service, except:

-  ``object_type``

The **service** key is optional. When it is included in the action
payload, the executor will invoke the ``process check results`` call to
set the status of a service; otherwise, it will set the one of a host.

An example of a valid Tornado Action is to set the status of the service
``myhost|myservice``:

.. code:: json

       {
         "id": "smart_monitoring_check_result",
         "payload": {
           "check_result": {
             "exit_status": "2",
             "plugin_output": "Output message"
           },
           "host": {
             "object_name": "myhost",
             "address": "127.0.0.1",
             "check_command": "hostalive",
             "vars": {
               "location": "Rome"
             }
           },
           "service": {
              "object_name": "myservice",
              "check_command": "ping"
           }
         }
       }

By simply removing the ``service`` key, the same action will set the
status of the host ``myhost``:

.. code:: json

        {
          "id": "smart_monitoring_check_result",
          "payload": {
            "check_result": {
              "exit_status": "2",
              "plugin_output": "Output message"
            },
            "host": {
              "object_name": "myhost",
              "address": "127.0.0.1",
              "check_command": "hostalive",
              "vars": {
                "location": "Rome"
              }
            }
          }
        }

The flowchart shown in :numref:`fig-monitoring-executor-flowchart`
helps to understand the behaviour of the Monitoring Executor in
relation to Icinga2 and Icinga Director REST APIs.

Retry logic
+++++++++++

When a new object is created, after the call to the
``process_check_result`` the executor calls the Icinga ``/v1/objects``
API to check whether the new object is still in ``PENDING`` state. In
case the object is found to be pending, the executor will call again the
``process_check_result`` API, for a predefined number of attempts, until
the check to the object state returns that it is not ``PENDING``
anymore.

