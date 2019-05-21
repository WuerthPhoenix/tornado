# Email Collectors

The Email Collector receives a MIME email message as input, parses it and produces a Tornado Event.


## EmailEventCollector

The _EmailEventCollector_ expects to receive a valid [MIME email message](https://en.wikipedia.org/wiki/MIME) as
input, for example:

```
{
  "type": "email",
  "created_ms": 1554130814854,
  "payload":{
    "subject": "Email subject",
    "body": "Email body",
    "other": {
      "some_text": "some text",
      "a_bool": true,
      "a_number": 123456.789,
      "something_else": {}
    }
  }
}
```

The above email will generate this Event:

```json
{
  "type": "email",
  "created_ms": 1554130814854,
  "payload": {
    "@timestamp": "2018-11-01T23:59:59+01:00",
    "host": "neteye01",
    "hostgroups": [
      "windows",
      "database",
      "rome"
    ],
    "icinga_customfields": {
      "snmpcommunity": "secret",
      "os": "windows"
    },
    "severity": "DEBUG",
    "facility": "daemon",
    "syslog-tag": "nfcapd[20747]:",
    "source": "nfcapd",
    "message": " Process_v9: Found options flowset: template 259"
  }
}
```
