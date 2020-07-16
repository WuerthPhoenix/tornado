# Monitoring Executor

The Monitoring Executor is an executor that permits to perform Icinga `process check results` 
also in the case that the Icinga object for which you want to perform the `process check result`
does not yet exist.

This is done by means of executing the action `process check result` with the Icinga Executor, 
and by executing the actions `create_host`/`create_service` with the Director Executor, in case
the underlying Icinga objects do not yet exist in Icinga.

## How It Works

This executor expects a Tornado Action to include the following elements in its payload:

1. An __action_name__: The Monitorin action to perform.
1. An __process_check_result_payload__: The payload of the Director action.
1. An __host_creation_payload__: Boolean value, which determines whether to create the
1. An __service_creation_payload__: The payload of the Director action.
 specified Icinga Object also in Icinga2 (mandatory only in case __action_name__ is 
 `create_and_or_process_service_passive_check_result`). 

Valid values for __action_name__ are:
* __create_and_or_process_host_passive_check_result__: sets the `passive check result` for a `host`, and creates the
host if necessary
* __create_and_or_process_service_passive_check_result__: sets the `passive check result` for a `service`, and creates
the underlying host and service if necessary

The __process_check_result_payload__ should contain at least all mandatory parameters expected by the
Icinga API to perform the action. The object on which you want to set the `passive check result` must be specified
with the field `host` in case of action __create_and_or_process_host_passive_check_result__, and `service` in case of
action __create_and_or_process_service_passive_check_result__ (e.g. specifying a set of objects on which to apply the
`passive check result` with the parameter `filter` is not valid)

The __host_creation_payload__ should contain at least all mandatory parameters expected by the Icinga Director REST API
to perform the creation of a host.

The __servie_creation_payload__ should contain at least all mandatory parameters expected by the Icinga Director REST API
to perform the creation of a service.

An example of a valid Tornado Action is:
```json
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
```
