# Director Executor

The Director Executor is an application that extracts data from a Tornado Action and prepares it to be
sent to the [Icinga Director REST API](https://icinga.com/docs/director/latest/doc/70-REST-API/).


## How It Works

This executor expects a Tornado Action to include the following elements in its payload:

1. An __action_name__: The Director action to perform.
1. An __action_payload__ (optional): The payload of the Director action.
1. An __icinga2_live_creation__ (optional): Boolean value, which determines whether to create the
 specified Icinga Object also in Icinga2.

Valid values for __action_name__ are:
* __create_host__: creates an object of type `host` in the Director
* __create_service__: creates an object of type `service` in the Director

The __action_payload__ should contain at least all mandatory parameters expected by the
Icinga Director REST API for the type of object you want to create.


An example of a valid Tornado Action is:
```json
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
```

The Director Executor is only in charge of extracting the required data from the Tornado Action
payload. It will not perform any HTTP calls to the Director API, which instead should be taken care
of by those using the executor.
