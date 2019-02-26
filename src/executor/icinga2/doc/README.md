# Icinga2 Executor

The Icinga2 Executor is an executor that extracts data from a Tornado Action and prepares it to be
ingested by the [Icinga2 API](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api).


## How It Works

This collector expects a Tornado Action to include the following elements in the payload:

1. An __icinga2_action_name__: The Icinga2 action to perform.
1. An __icinga2_action_payload__ (optional): The payload of the Icinga2 action.

The __icinga2_action_name__ should match one of the existent
[Icinga2 actions](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#actions).

The __icinga2_action_payload__ should contain at least all the mandatory fields forseen by the
specific Icinga2 action.


An example of a valid Tornado Action is:
```json
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
```


The Icinga2 Executor is only in charge of extracting the required data from the Tornado Action
payload; it will not perform any HTTP call to the Icinga2 API, the executor users should take
care of it.