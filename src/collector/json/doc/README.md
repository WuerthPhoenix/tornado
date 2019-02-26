# JSON Collectors

These are Collectors that receive an input in JSON and unmarshall it into an internal Event struct.

There are currently two available implementations:
1. The _JsonEventCollector_
1. The _JsonPayloadCollector_



## JsonEventCollector

The _JsonEventCollector_ expects to receive a valid JSON representation of a Tornado Event as
input. It is used internally by Tornado to unmarshall Events received, for example, from a TCP or
UDS socket.

The JSON input format should respect the Event structure, for example:

```json
{
  "type": "email",
  "created_ts": "2018-11-28T21:45:59.324310806+09:00",
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



## JsonPayloadCollector

The _JsonPayloadCollector_ receives any valid JSON object and creates a Tornado Event whose
payload is that input. For example, the following input:

```json
{
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
```

will generate this Event:

```json
{
  "type": "event_type_from_config",
  "created_ts": "2018-11-28T21:45:59.324310806+09:00",
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

The Event "type" property must be specified when the collector is instantiated.
