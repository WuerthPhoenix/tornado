# <a id="tornado-howto-simulate"></a> How To Simulate Events

This How To is intended to help you learn how to simulate incoming events (such as monitoring
events, network events, email or SMS messages) in order to test that the rules you configure
will properly match those events and correctly invoke the chosen actions.

Before continuing, you should first check the
[prerequisites for Tornado](/neteye/doc/module/tornado/chapter/tornado-howto-overview).



## <a id="tornado-howto-simulate-step1"></a> Step #1:  Checking for Rule Matches

First let's test whether we can correctly match a rule.  To do this, we will purposefully not
execute any actions should a rule match.  This capability is designed into Tornado's Event
Simulator by setting the *process_type* to **SkipActions**.  The possible values for
*process_type* are:
* **Full:** the event is processed and linked actions are executed
* **SkipActions:** the event is processed but actions are not executed

So let's put that in our JSON request, and add some dummy values for the required fields.
We'll pipe the results through the **jq** utility for now so that we can more easily interpret
the results:

```
# curl -H "content-type: application/json" \
       -X POST -vvv \
       -d '{"event":{"type":"something", "created_ms":111, "payload": {}}, "process_type":"SkipActions"}' \
       http://localhost:4748/api/send_event | jq .
* About to connect() to localhost port 4748 (#0)
*   Trying ::1...
* Connection refused
*   Trying 127.0.0.1...
* Connected to localhost (127.0.0.1) port 4748 (#0)
> POST /api/send_event HTTP/1.1
> User-Agent: curl/7.29.0
> Host: localhost:4748
> Accept: */*
> content-type: application/json
> Content-Length: 93
>
* upload completely sent off: 93 out of 93 bytes
< HTTP/1.1 200 OK
< content-length: 653
< content-type: application/json
< date: Mon, 20 May 2019 15:22:35 GMT
<
{ [data not shown]
100   746  100   653  100    93  12933   1841 --:--:-- --:--:-- --:--:-- 12803
* Connection #0 to host localhost left intact
{
  "event": {
    "type": "something",
    "created_ms": 111,
    "payload": {}
  },
  "result": {
    "type": "Rules",
    "rules": {
      "rules": {
        "all_emails": {
          "rule_name": "all_emails",
          "status": "NotMatched",
          "actions": [],
          "message": null
        },
        "emails_with_temperature": {
          "rule_name": "emails_with_temperature",
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
                  "created_ms": 111,
                  "payload": {},
                  "type": "something"
                }
              }
            }
          ],
          "message": null
        },
        "icinga_process_check_result": {
          "rule_name": "icinga_process_check_result",
          "status": "NotMatched",
          "actions": [],
          "message": null
        }
      },
      "extracted_vars": {}
    }
  }
}
```

What we sent is copied in the *event* field.  The result of the matching process is the value in
the *results* field.  If you look at the *rule_name* fields, you can see the four rules that were
checked:  *all_emails*, *emails_with_temperature*, *archive_all*, and *icinga_process_check_result*.

Looking at the *status* field, we can see that only the *archive_all* rule matched our incoming
event, while the remaining rules have *status* **NotMatched** with empty *actions* fields.  The
*actions* field for our matched rule can help inform us what would happen if we had selected the
*process_type* **Full** instead of **SkipActions**.



## <a id="tornado-howto-simulate-step2"></a> Step #2:  Actions after Matches

If we repeat the same command as above, but with *process_type* set to **Full**, then that
*Archive* action will be executed.  We won't repeat that command here because on a production
system, it can be dangerous to execute an action unless you know what you are doing.  The effects
of poorly configured actions can include shutting down your entire monitoring server, or crashing
the server or VM.

If you have configured your *Archive Executor*, we can now check the results of running that
command.  This executor is relatively safe, as it just writes the input event into a log file.
So for instance if we configured the executor to save data in */test/all.log*, we should be
able to see the output immediately:
```
# cat /neteye/shared/tornado/data/archive/test/all.log
{"type":"something","payload":{},"created_ms":111}
```
