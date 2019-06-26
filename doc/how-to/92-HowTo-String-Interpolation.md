# <a id="tornado-howto-string-interpolation"></a> How To Use the Icinga2 Executor with String Interpolation

This advanced How To is intended to help you configure, use and test the **Icinga2 Executor**
in combination with the **String Interpolation** feature, creating a **passive check only**
monitoring service result with dynamic generation of the check **result content**. The general
approach, however, can be used to execute icinga2 actions ( or any other action )
dynamically based on the event content.

Before continuing, you should first check the
[prerequisites for Tornado](/neteye/doc/module/tornado/chapter/tornado-howto-overview).


## <a id="tornado-howto-string-interpolation-step1"></a> Step #1:  Prerequisites

**Tornado:**
* For testing purposes, we will manually send an event to tornado via CLI. In a production
  environment, tornado can accept events from any collector and even via an HTTP POST request
  with JSON payload.
* Make sure that the username and password are properly set to your dedicated tornado user in icinga2
  ```
  /neteye/shared/tornado/conf/icinga2_client_executor.toml
  ```

**Icinga Director / Icinga2**:
* Create a **host** called *host.example.com* with no particular requirements
* Create a **service template** with the following properties:
    * Check command: *dummy*
    * Execute active checks: *No*
    * Accept passive checks: *Yes*
* Create a **service** called *my_dummy* on the host *host.example.com* importing the
  previously created service template
* Deploy this configuration to icinga2

## <a id="tornado-howto-string-interpolation-step2"></a> Step #2:  Service and Rule Configuration

This is an example of an event, which we'll use later on, sending it to tornado. For now, keep it in handy while reading the next section, as the rules are based on this specific format:
```json
{
  "type": "dummy_passive_check",
  "created_ms": 1000,
  "payload": {
    "hostname": "host.example.com",
    "service": "my_dummy",
    "exit_status": "2",
    "measured": {
        "result1": "0.1",
        "result2": "98"
    }
  }
}
```

Now let's configure a rule with the following **WHERE** constraints:
* events of *type* **dummy_passive_check**
* containing the critical *exit_code* **2**
* and *service name* **my_dummy**

We can achieve this by creating the following rule in `/neteye/shared/tornado/conf/rules.d/` in a file called 
*900_icinga2_my_checkresult_crit.json*
```json
{
  "description": "Set the critical status for my_dummy checks in Icinga2",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "AND",
      "operators": [
        {
          "type": "equal",
          "first": "${event.type}",
          "second": "dummy_passive_check"
        },
        {
          "type": "equal",
          "first": "${event.payload.service}",
          "second": "my_dummy"
        },
        {
          "type": "equal",
          "first": "${event.payload.exit_status}",
          "second": "2"
        }
      ]
    },
    "WITH": {}
  },
  "actions": []
}
```

In addition, we want our rule to trigger an icinga2 action with a passive check result that:
* Applies to the *my_dummy* service of the host in **${event.payload.hostname}**
* Sets the **exit_status** to *critical* (=2)
* Adds a human readable **plugin_output**
* Adds a machine readable **performance_data** with two simple static thresholds:
    * **result1** perfdata: it contains immaginary millisecond duration, with 300ms _warn_ and 500ms _crit_ threshold
    * **result2** perfdata: it contains immaginary percentage, with 80% _warn_ and 95% _crit_


Here's our final rule including the desired action:
```json
{
  "description": "Set the critical status for my_dummy checks in Icinga2",
  "continue": true,
  "active": true,
  "constraint": {
    "WHERE": {
      "type": "AND",
      "operators": [
        {
          "type": "equal",
          "first": "${event.type}",
          "second": "dummy_passive_check"
        },
        {
          "type": "equal",
          "first": "${event.payload.service}",
          "second": "my_dummy"
        },
        {
          "type": "equal",
          "first": "${event.payload.exit_status}",
          "second": "2"
        }
      ]
    },
    "WITH": {}
  },
  "actions": [
    {
      "id": "icinga2",
      "payload": {
        "icinga2_action_name": "process-check-result",
        "icinga2_action_payload": {
          "exit_status": "${event.payload.exit_status}",
          "plugin_output": "CRITICAL - Result1 is ${event.payload.measured.result1}ms Result2 is ${event.payload.measured.result2}%",
          "performance_data": "result_1=${event.payload.measured.result1}ms;300.0;500.0;0.0 result_2=${event.payload.measured.result2}%;80;95;0",
          "filter": "host.name==\"${event.payload.hostname}\" && service.name==\"${event.payload.service}\"",
          "type": "Service"
        }
      }
    }
  ]
}

```

Remember that whenever you create a new rule or edit an existing one,
you need to restart the Tornado service. It is also
helpful to run a check to make sure there are no syntactic errors in your new rule:
```
# tornado --config-dir=/neteye/shared/tornado/conf check
# systemctl restart tornado.service
```

If you performed all the previous steps correctly, you should notice that,
whenever an event matches the rule, 
the body of the generated action will no longer contain any of the 
original placeholders ${event.payload.*}. 
In fact, they are replaced by the actual values extracted from the event.
If one or more placeholders cannot be resolved, the action will fail.


## <a id="tornado-howto-string-interpolation-step3"></a> Step #3:  Send the event and set the status

Open a browser and verify that you deployed the required configuration to icinga2,
this can be done navigating to the
**Overview > Services > host.example.com: my_dummy** [service](/neteye/monitoring/service/show?host=host.example.com&service=my_dummy). You should see that it is still in
**Pending** state as no active checks are executed.

We can now use the *tornado-send-event* helper command to send the JSON content of a file to the tornado API.
So, create a file called *payload.json* with the following contents in your *home directory*:
```json
{
  "type": "dummy_passive_check",
  "created_ms": 1000,
  "payload": {
    "hostname": "host.example.com",
    "service": "my_dummy",
    "exit_status": "2",
    "measured": {
        "result1": "0.1",
        "result2": "98"
    }
  }
}
```

Send it to tornado using the following command:
```
tornado-send-event ~/payload.json
```

This should trigger our rule and produce a response similar to the following:
```json
{
  "event": {
    "type": "dummy_passive_check",
    "created_ms": 1000,
    "payload": {
      "service": "my_dummy",
      "measured": {
        "result1": "0.1",
        "result2": "98"
      },
      "exit_status": "2",
      "hostname": "host.example.com"
    }
  },
  "result": {
    "type": "Rules",
    "rules": {
      "rules": {
        [...omitted...]
        "icinga2_my_checkresult_crit": {
          "rule_name": "icinga2_my_checkresult_crit",
          "status": "Matched",
          "actions": [
            {
              "id": "icinga2",
              "payload": {
                "icinga2_action_name": "process-check-result",
                "icinga2_action_payload": {
                  "exit_status": "2",
                  "filter": "host.name==\"host.example.com\" && service.name==\"my_dummy\"",
                  "performance_data": "result_1=0.1ms;300.0;500.0;0.0 result_2=98%;80;95;0",
                  "plugin_output": "CRITICAL - Result1 is 0.1ms Result2 is 98%",
                  "type": "Service"
                }
              }
            }
          ],
          "message": null
        },
        [...omitted...]
      "extracted_vars": {}
    }
  }
}
```

Now open the browser and check the Service in Icinga2 again, you'll see that it has **NOT** changed yet.
This is intentional, in fact, to avoid triggering actions accidentally,
the _tornado-send-event_ command executes no actions by default. 
We can tell tornado to actually
execute the actions by passing the **-f** flag to the script as follows:
```
tornado-send-event ~/payload.json -f
```

Checking the Service once again should show that it turned red and its state is *soft critical*. Depending
on your configuration, after a few additional executions, it will end up in hard *critical* state.

As you may note, if we change the *exit_code* in the event payload to
anything other than *2*, the rule will not match as we filter out everything
but critical events.
Adding another rule that filters only on *OK* states (exit_code == 0) and then sets the service state to an OK state, is left as an exercise to the reader.
