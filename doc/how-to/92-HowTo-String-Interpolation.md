# <a id="tornado-howto-string-interpolation"></a> How To Use the Icinga2 Executor with String Interpolation

This advanced How To is intended to help you configure, use and test the **Icinga2 Executor**
in combination with the **String Interpolation** feature, creating a **passive check only**
monitoring service result with dynamic generation of the check **result content**. The general
approach however can be used to execute icinga2 actions ( or any action really )
dynamically based on the event content.

Before continuing, you should first check the
[prerequisites for Tornado](/neteye/doc/module/tornado/chapter/tornado-howto-overview).


## <a id="tornado-howto-string-interpolation-step1"></a> Step #1:  Prerequisites

**Tornado:**
* For testing purposes we will manually send an event to tornado via CLI. In a production
  environment tornado can accept events from any collector and even via HTTP POST request
  with JSON payload.
* Make sure that the username and password are set to your dedicated tornado user in icinga2
  ```
  /neteye/shared/tornado/conf/icinga2_client_executor.toml
  ```

**Icinga Director / Icinga2**:
* Create a **host** called *host.example.com* without any particular requirements
* Create a **service template** with the following properties:
    * Check command: *dummy*
    * Execute active checks: *No*
    * Accept passive checks: *Yes*
* Create a **service** called *my_dummy* on the host *host.example.com* importing the
  previously created service template
* Deploy this configuration to icinga2

## <a id="tornado-howto-string-interpolation-step2"></a> Step #2:  Service and Rule Configuration

We will send an event of the following form to tornado, for now you do not need to do anything with
it, should make the following section a lot easier to understand:
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

Now let's configure a rule with the following **WHERE** constarints:
* events of *type* **dummy_passive_check**
* containing the critical *exit_code* **2**
* and our *service name* **my_dummy**

You can achieve this by simply createing the following rule in `/neteye/shared/tornado/conf/rules.d/` in a file called like
*900_icinga2_my_checkresult_crit.json*
```json
{
  "name": "icinga2_my_checkresult_crit",
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

Our rule needs to then trigger an icinga2 action submitting a passive check result:
* Apply it to the *my_dummy* service of the host in **${event.payload.hostname"}**
* Setting the **exit_status** to *critical* (=2)
* Adding human readable **plugin_output**
* Adding machine readable **performance_data** with ( for simplicity reasons ) static thresholds
    * The **result1** perfdata contains immaginary millisecond duration, with 300ms warn and 500ms crit threshold
    * The **result2** perfdata contains immaginary percentage, with 80% warn and 95% crit


Here's our new rule containing both parts:
```json
{
  "name": "icinga2_my_checkresult_crit",
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
You will see, that whenever you send an event via API to Tornado, that the returned result will no longer
contain the placeholders of **${event.payload.xy}**, but the actual values of the event if applicable.
If some of them are not contained in the event, the action will fail.

Remember that whenever you create a new rule, you will need to restart the Tornado service.  It's always
helpful to run a check first to make sure there are no syntactic errors in your new rule:
```
# tornado --config-dir=/neteye/shared/tornado/conf check
# systemctl restart tornado.service
```

## <a id="tornado-howto-string-interpolation-step3"></a> Step #3:  Send the event and set the status

First open a browser and check that you deployed the configuration to icinga2 navigating to the
**Overview > Services > host.example.com: my_dummy** [service](/neteye/monitoring/service/show?host=host.example.com&service=my_dummy). You will see that it is still in
**Pending** state as no active checks are executed.

You can use the *tornado-send-event* helper command to send the contents of a file to tornado.
Do this by creating a file called *payload.json* with the following contents in your *home directory*:
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

The response should be similar to the following:
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

If you check the Service again via Browser, you'll see that it has **NOT** changed yet. This
is intentional, as the API will not execute any actions by default. This is to avoid triggering
actions accidentally during testing.

Now that we're sure that our rule is the only one *Matched*, we can tell tornado to actually
execute the actions by passing the **-f** flag to the script as follows:
```
tornado-send-event ~/payload.json -f
```

Now you will see that the service turned red and has gone into *soft critical* state. Depending
on your configuration, after a few additional executions, it will end up in hard *critical* state.

As you may note fiddling with the event content, if you change the *exit_code* in the payload to
anything other than *2*, nothing will happen, as the rule filters out all but critical events.
Adding another rule filtering on only the *OK* (exit_code == 0) states and thus setting the service
state to an OK state, is left as an  exercise to the reader.
