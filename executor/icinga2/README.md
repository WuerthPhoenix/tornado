# Icinga2 Executor

The Icinga2 Executor is an executor that extracts data from a Tornado Action and prepares it to be
sent to the [Icinga2 API](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api).



## How It Works

This executor expects a Tornado Action to include the following elements in its payload:

1. An __icinga2_action_name__: The Icinga2 action to perform.
1. An __icinga2_action_payload__ (optional): The parameters of the Icinga2 action.

The __icinga2_action_name__ should match one of the existing
[Icinga2 actions](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#actions).

The __icinga2_action_payload__ should contain at least all mandatory parameters expected by the
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
payload. It will not perform any HTTP calls to the Icinga2 API, which instead should be taken care
of by those using the executor.
