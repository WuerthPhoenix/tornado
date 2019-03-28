# SNMP Trap Daemon Collector

The snmptrapd_collector is an embedded perl trap handling for Net-SNMP's snmptrapd.
When registered as a subroutines into the Net-SNMP snmptrapd process, 
it receives snmptrap-specific inputs, 
transforms them into Tornado Events, and forwards them to the TCP address 
of the Tornado engine.

## Configuration

### Prerequisites

This collector has the following runtime requirements:
- Perl 5.16 or greater
- Perl packages required:
  - DateTime
  - JSON
  - NetSNMP::TrapReceiver

you can verify that the Perl packages are available with the command:
```bash
> perl -e 'use JSON;' && \
  perl -e 'use NetSNMP::TrapReceiver;' && \
  perl -e 'use DateTime;'
```

If no messages are displayed in the console, then everything's ok; otherwise, 
you'll see some error messages.

In case of missing dependencies, use your system package-manager to install them.

For example, the required Perl packages can be installed on an Ubuntu system with:
```bash
> sudo apt install libdatetime-perl libjson-perl libsnmp-perl
```

### Activation

This Collector is meant to be integrated with snmptrapd.

To activate it, put the following line in your snmprapd.conf file:

```
perl do "/path_to_the_script/snmptrapd_collector.pl"; 
```

Consequently, it is never started manually, but instead will be started, and managed,
directly by snmptrapd itself.

At startup, if the collector is configured properly, you should see 
this entry in the logs or in the daemon standard error:
```
The snmptrapd_collector was loaded successfully.
```


## How It Works

The snmptrapd_collector receives snmptrapd messages, parses them, generates Tornado Events
and, finally, sends them to the Tornado TCP events socket.

The current version, will always use these hardcoded values to find the Tornado engine:
- Tornado engine IP address: _127.0.0.1_
- Tornado engine port: _4747_ 

The perl script should automatically reconnect in case the Tornado engine is not 
temporarily not available.

 
Given a snmptrapd messages that contains the following information:
```
PDU INFO:
  version                        1
  errorstatus                    0
  community                      public
  receivedfrom                   UDP: [127.0.1.1]:41543->[127.0.2.2]:162
  transactionid                  1
  errorindex                     0
  messageid                      0
  requestid                      414568963
  notificationtype               TRAP
VARBINDS:
  iso.3.6.1.2.1.1.3.0            type=67 value=Timeticks: (1166403) 3:14:24.03
  iso.3.6.1.6.3.1.1.4.1.0        type=6  value=OID: iso.3.6.1.4.1.8072.2.3.0.1
  iso.3.6.1.4.1.8072.2.3.2.1     type=2  value=INTEGER: 123456
```

this collector will produce this Tornado Event:
```json
{
   "type":"snmptrapd",
   "created_ts":"2019-03-28T09:38:10Z",
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

The structure of the generated Event is not configurable.

# Testing

To test the collector on a machine, verify that snmptrapd is installed and
follow the above collector configuration instructions.

As a prerequisite, the Tornado Engine should be up and running on the same machine 
([See the dedicated Tornado engine documentation](../../engine/doc/README.md)). 

In addition, to send fake snmptrapd messages, the _snmptrap_ tool is required.

On Ubuntu, both the _snmptrap_ tool and the _snmptrapd_ daemon can be installed with:
```bash
sudo apt install snmp snmptrapd
```

You can now start snmptrapd (as root) in a terminal:
```bash
> snmptrapd -f -Le
```

And send fake messages with the command:
```bash
> snmptrap -v 2c -c public localhost '' 1.3.6.1.4.1.8072.2.3.0.1 1.3.6.1.4.1.8072.2.3.2.1 i 123456
```

If everything is configured correctly, you should a the message in the snmptrapd stardard error
and an Event of type _'snmptrapd'_ received by Tornado Engine. 


