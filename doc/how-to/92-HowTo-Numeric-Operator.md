# <a id="tornado-howto-numerical-operator"></a> How To Use the Numerical Operators

This How To is intended to help you configure, use and test rules involving the numerical
operators, allowing you to compare quantities in a given Event either to each other or
to a constant in the rule.  One important use case is in IoT, where you may be remotely
measuring temperatures, humidities, and other physically measurable quantities in order
to decide whether to shut down a server.

Before continuing, you should first make sure the
[prerequisites for Tornado](/neteye/doc/module/tornado/chapter/tornado-howto-overview)
are satisfied.



## <a id="tornado-howto-numerical-operator-step1"></a>  Step #1:  Simulating Rising Temperatures

Because IoT hardware and its reporting software differ significantly from one installation to
another, we will use the
[Event Simulator](/neteye/doc/module/tornado/chapter/tornado-howto-simulate)
to simulate a rising series of temperatures resulting in an action to shut down a server.

```
# curl -H "content-type: application/json" \
       -X POST -vvv \
       -d '{"event":{"type":"something", "created_ms":111, "payload": {}}, "process_type":"SkipActions"}' \
       http://localhost:4748/api/send_event | jq .
```

<!-- Put a temperature in the payload -->



## <a id="tornado-howto-numerical-operator-step2"></a> Step #2:  Configuring a Rule with Comparisons


* Discuss (and eventually link to) the numerical operators in the Tornado doc
* Use the email temperature rule as an example?

To start, let's create a rule that matches all incoming SNMP Trap events, extracts the source IP
field, and uses the **Archive Executor** to write the entire event into a log file in a directory
named for the source IP (this would allow us to keep events from different network devices in
different log directories).   The SNMP Trap Collector produces a JSON structure, which we will
serialize to write into the file defined in Step #3.
<!-- Try to link to the SNMP Trap Collector documentation -->

A JSON structure representing an incoming SNMP Trap Event looks like this:
```json
{
   "type":"snmptrapd",
   "created_ms":"1553765890000",
   "payload":{
      "protocol":"UDP",
      "src_ip":"127.0.1.1",
      "src_port":"41543",
      "dest_ip":"127.0.2.2",
      "PDUInfo":{
         "version":"1",
         "errorstatus":"0",
         "community":"public",
         "receivedfrom":"UDP: [127.0.1.1]:41543->[127.0.2.2]:162",
         "transactionid":"1",
         "errorindex":"0",
         "messageid":"0",
         "requestid":"414568963",
         "notificationtype":"TRAP"
      },
      "oids":{
         "iso.3.6.1.2.1.1.3.0":"67",
         "iso.3.6.1.6.3.1.1.4.1.0":"6",
         "iso.3.6.1.4.1.8072.2.3.2.1":"2"
      }
   }
}
```

So our rule needs to match incoming events of type *snmptrapd*, and when one matches, extract the
**src_ip** field from the **payload** array.  Although the rules used when Tornado is running are
found in */neteye/shared/tornado/conf/rules.d/*, we'll model our rule based on one of the example
rules found here:
```
/usr/lib64/tornado/examples/rules/
```

Since we want to match any SNMP event, let's adapt the matching part of the rule found in
*/usr/lib64/tornado/examples/rules/001_all_emails.json*.  And since we want to run the
*archive* executor, let's adapt the action part of the rule found in
*/usr/lib64/tornado/examples/rules/010_archive_all.json*.

Here's our new rule containing both parts:
```
{
    "name": "all_snmptraps",
    "description": "This matches all snmp events",
    "continue": true,
    "active": true,
    "constraint": {
      "WHERE": {
        "type": "AND",
        "operators": [
          {
            "type": "equal",
            "first": "${event.type}",
            "second": "snmptrapd"
          }
        ]
      },
      "WITH": {}
    },
    "actions": [
      {
        "id": "archive",
        "payload": {
          "event": "${event}",
          "source": "${event.payload.src_ip}",
          "archive_type": "trap"
        }
    }
    ]
  }
```

Changing the "second" field of the WHERE constraint as above will cause the rule to match with any
SNMP event.  In the "actions" section, we add the "source" field which will extract the source IP,
and change the archive type to "trap".  We'll see why in Step #3.

Remember to save our new rule where Tornado will look for active rules, which in the default
configuration is */neteye/shared/tornado/conf/rules.d/*.  Let's give it a name like
*030_snmp_to_archive.json*.

Also remember that whenever you create a new rule and save the file in that directory, you will
need to restart the Tornado service.  And it's always helpful to run a check first to make sure
there are no syntactic errors in your new rule:
```
# tornado --config-dir=/neteye/shared/tornado/conf check
# systemctl restart tornado.service
```



## <a id="tornado-howto-snmp-collector-step3"></a> Step #3:  Configure the Archive Executor

<!-- We could use a link to the description of Archive Event. -->

If you look at the file */neteye/shared/tornado/conf/archive_executor.toml*, which is the
configuration file for the **Archive Executor**, you will see that the default base archive path
is set to */neteye/shared/tornado/data/archive/*.  Let's keep the first part, but under
"[paths]" let's add a specific directory (relative to the base directory given for "base_path".
This will use the keyword "trap", which matches the "archive_type" in the "action" part of our
rule from Section #3, and will include our "source" field, which extracted the source IP from
the original event's payload:

```
base_path =  "/neteye/shared/tornado/data/archive/"
default_path = "/default/default.log"
file_cache_size = 10
file_cache_ttl_secs = 1

[paths]
"trap" = "/trap/${source}/all.log"
```

Combining the base and specific paths yields the full path where the log file will be saved
(automatically creating directories if necessary), with our "source" variable instantiated.
So if the source IP was 127.0.0.1, the log file's name will be:
```
/neteye/shared/tornado/data/archive/trap/127.0.0.1/all.log
```

When an SNMP event is received, the field "event" under "payload" will be written into that
file.  Since we have only specifed "event", the entire event will be saved to the log file.



## <a id="tornado-howto-snmp-collector-step4"></a> Step #4:  Watch Tornado "in Action"

Let's observe how our newly configured SNMP Trap Collector works using a bash shell.  If you want
to see what happens when an event is processed, open two separate shells to:
* Show internal activity in the matcher engine 
* Send SNMP events manually, and display the results

In the first shell, run the following command to see the result of rule matches in real-time:
```
# journalctl -f -u snmptrapd
```

In the second shell, we will manually initiate simulated SNMP Trap events like this:
```
# snmptrap -v 2c -c public localhost '' 1.3.6.1.4.1.8072.2.3.0.1 1.3.6.1.4.1.8072.2.3.2.1 i 123456
```

What you should see is that when the SNMP event is initiated in the second shell, output appears
in the first shell, indicating that the event has been successfully matched.  In addition, we
can now look at the result of the match by looking at the log file configured by the *archive*
executor.

There you should see the full event written into the file we specified during Step #3:
```
/neteye/shared/tornado/data/archive/trap/127.0.0.1/all.log
```



## <a id="tornado-howto-snmp-collector-wrapup"></a> Wrapping Up

That's it!  You've successfully configured Tornado to respond to SNMP trap events by logging
them in a directory specific to each network device.

You can also use different executors, such as the **Icinga 2 Executor**, to send SNMP Trap events
as monitoring events straight to Icinga 2 where you can see the events in a NetEye dashboard.  The
[Icinga documentation](https://icinga.com/docs/icinga2/latest/doc/12-icinga2-api/#actions)
shows you which commands the executor must implement to achieve this.
