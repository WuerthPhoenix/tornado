# <a id="tornado-howto-email-collector"></a> How To Use the Email Collector

This How To is intended to help you configure, use and test the Email Collector
in your existing NetEye Tornado installation. We will configure a rule, such that
we log all incoming mail subjects in a log file per *from* email.

Before continuing, you should first check the
[prerequisites for Tornado](/neteye/doc/module/tornado/chapter/tornado-howto-overview).

<!-- Add summary of input and expected output -->



## <a id="tornado-howto-email-collector-step1"></a> Step #1:  Email and Package Configuration

* For testing purposes we will use **mailx** to send mails on the local machine.
* If you just upgraded your tornado installation run *neteye-secure-install* and
* Make sure the Email Collector service is running:
  ```
  ● tornado_email_collector.service - Tornado Email Collector - Data Collector for procmail
   Loaded: loaded (/usr/lib/systemd/system/tornado_email_collector.service; disabled; vendor preset: disabled)
  Drop-In: /etc/systemd/system/tornado_email_collector.service.d
           └─neteye.conf
   Active: active (running) since Thu 2019-06-20 19:08:53 CEST; 20h ago
  ```
* Send an email to the dedicated **eventgw** user which will then be processed by tornado
  ```
  # echo "TestContent" | mail -s TestSubject eventgw@localhost
  ```
* Now test that a sent email makes it to Tornado (the timestamp reported by journalctl should be
  at most a second or two after you send the email):
  ```
  # journalctl -u tornado_email_collector.service
  Jun 21 15:11:59 host.example.com tornado_email_collector[12240]: [2019-06-21][15:11:59][tornado_common::actors::uds_server][INFO] UdsServerActor - new client connected to [/var/run/tornado/email.sock]
  ```


## <a id="tornado-howto-email-collector-step2"></a> Step #2:  Service and Rule Configuration

Now let's configure a simple rule that just archives the subject and sender of an email
into a log file.

```
Here is an example of an Event created by the Email Collector:
<!-- See the doc at src/collector/email/doc/README.md -->

```json
{
  "type": "email",
  "created_ms": 1554130814854,
  "payload": {
    "date": 1475417182,
    "subject": "This is a test email",
    "to": "email1@example.com",
    "from": "email2@example.com",
    "cc": "",
    "body": "This is the plaintext version, in utf-8. Proof by Euro: €",
    "attachments": []
  }
}
```

Our rule needs to match incoming events of type *email*, and when one matches, extract the
**subject** field and the **from** field (sender)  from the **payload** object.  Rules used
when Tornado is running are found in */neteye/shared/tornado/conf/rules.d/*, but we'll
model our rule based on one of the example rules found here:
```
/usr/lib64/tornado/examples/rules/
```

Since we want to match any email event, let's adapt the matching part of the rule found in
*/usr/lib64/tornado/examples/rules/001_all_emails.json*.  And since we want to run the
*archive* executor, let's adapt the action part of the rule found in
*/usr/lib64/tornado/examples/rules/010_archive_all.json*.

Here's our new rule containing both parts:
```
{
    "name": "all_email_messages",
    "description": "This matches all email messages, extracting sender and subject",
    "continue": true,
    "active": true,
    "constraint": {
      "WHERE": {
        "type": "AND",
        "operators": [
          {
            "type": "equal",
            "first": "${event.type}",
            "second": "email"
          }
        ]
      },
      "WITH": {}
    },
    "actions": [
      {
        "id": "archive",
        "payload": {
          "sender": "${event.payload.from}",
          "subject": "${event.payload.subject}",
          "archive_type": "archive_mail"
        }
      }
    ]
}
```

Changing the "second" field of the WHERE constraint as above will cause the rule to match with any
*email* event.  In the "actions" section, we add the "sender" field which will extract the "from"
field in the email, the "subject" field to extract the subject, and change the archive type to
"my_email_type".  We'll see why in Step #3.

Remember to save our new rule where Tornado will look for active rules, which in the default
configuration is */neteye/shared/tornado/conf/rules.d/*.  Let's give it a name like
*030_mail_to_archive.json*.

Also remember that whenever you create a new rule and save the file in that directory, you will
need to restart the Tornado service.  And it's always helpful to run a check first to make sure
there are no syntactic errors in your new rule:
```
# tornado --config-dir=/neteye/shared/tornado/conf check
# systemctl restart tornado.service
```



## <a id="tornado-howto-snmp-collector-step3"></a> Step #3:  Configure the Archive Executor

<!-- This section is copied from 92-HowTo-SNMP.md (maybe we should pull it out into a separate file?) -->

If you look at the file */neteye/shared/tornado/conf/archive_executor.toml*, which is the
configuration file for the **Archive Executor**, you will see that the default base archive path
is set to */neteye/shared/tornado/data/archive/*.  Let's keep the first part, but under
"[paths]" let's add a specific directory (relative to the base directory given for "base_path".
This will use the keyword "arhchive_mail", which matches the "archive_type" in the "action" part of our
rule from Section #3, and will include our "source" field, which extracted the source IP from
the original event's payload:

```
base_path =  "/neteye/shared/tornado/data/archive/"
default_path = "/default/default.log"
file_cache_size = 10
file_cache_ttl_secs = 1

[paths]
"archive_mail" = "/email/${sender}/extracted.log"
```

Combining the base and specific paths yields the full path where the log file will be saved
(automatically creating directories if necessary), with our "source" variable instantiated.
So if the source IP was 127.0.0.1, the log file's name will be:
```
/neteye/shared/tornado/data/archive/email/root/extracted.log
```

When an SNMP event is received, the field "event" under "payload" will be written into that
file.  Since we have only specified "event", the entire event will be saved to the log file.



## <a id="tornado-howto-email-collector-step4"></a> Step #4:  Check the Resulting Email Match

Let's see how our newly configured Email Collector works using a bash shell.

First we will again manually send an email to be intercepted by Tornado like this:
```
# echo "The email body." | mail -s "Test Subject" eventgw@localhost
```

Event processing should be almost immediate, so you can now look at the result of the match by
looking at the log file configured by the Archive executor.  There you should see the sender and
subject written into the file as we specified during Step #3:
```
/neteye/shared/tornado/data/archive/trap/127.0.0.1/all.log
```

And that's it!  You've successfully configured Tornado to process emails and logging the subject
and sender to a dynamic directory per sender.
