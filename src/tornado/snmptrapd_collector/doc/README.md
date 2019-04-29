# SNMP Trap Daemon Collector

The _snmptrapd_collector_ is an embedded Perl trap handler for Net-SNMP's snmptrapd.
When registered as a subroutine in the Net-SNMP snmptrapd process, it receives
snmptrap-specific inputs, transforms them into Tornado Events, and forwards them to
the TCP address of the Tornado Engine.

The implementation relies on the Perl NetSNMP::TrapReceiver package. You can refer to
[its documentation](https://metacpan.org/pod/NetSNMP::TrapReceiver)
for generic configuration examples and usage advice. 



## Configuration



### Prerequisites

This collector has the following runtime requirements:
- Perl 5.16 or greater
- Perl packages required:
  - Cpanel::JSON::XS
  - NetSNMP::TrapReceiver

You can verify that the Perl packages are available with the command:
```bash
$ perl -e 'use Cpanel::JSON::XS;' && \
  perl -e 'use NetSNMP::TrapReceiver;'
```

If no messages are displayed in the console, then everything is okay; otherwise, 
you will see error messages.

In case of missing dependencies, use your system's package manager to install them.

For example, the required Perl packages can be installed on an Ubuntu system with:
```bash
$ sudo apt install libcpanel-json-xs-perl libsnmp-perl
```



### Activation

This Collector is meant to be integrated with snmptrapd. To activate it, put the following line
in your _snmptrapd.conf_ file:

```
perl do "/path_to_the_script/snmptrapd_collector.pl"; 
```

Consequently, it is never started manually, but instead will be started, and managed,
directly by _snmptrapd_ itself.

At startup, if the collector is configured properly, you should see 
this entry either in the logs or in the daemon's standard error output:
```
The snmptrapd_collector was loaded successfully.
```



### Configuration options

The address of the Tornado Engine TCP instance to which the events are forwarded 
is configured with the following environment variables:
- __TORNADO_ADDR__: the IP address of Tornado Engine. If not specified, 
  it will use the default value _127.0.0.1_
- __TORNADO_PORT__: the port of the TCP socket of Tornado Engine. If not specified, 
  it will use the default value _4747_



## How It Works

The _snmptrapd_collector_ receives snmptrapd messages, parses them, generates Tornado Events
and, finally, sends them to the Tornado TCP events socket.

The received messages are kept in an in-memory non-persistent buffer that makes the application
resilient to Tornado Engine crashes or temporary unavailability.  When Tornado restarts, all
messages in the buffer will be sent.  When the buffer is full, the collector will start
discarding old messages.  The buffer max size is set to `10000` messages. 
 
Consider a snmptrapd message that contains the following information:
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

The collector will produce this Tornado Event:
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

The structure of the generated Event is not configurable.



# Testing

To test the collector, verify that snmptrapd is installed on the machine and
follow the collector configuration instructions above.

As a prerequisite, the Tornado Engine should be up and running on the same machine 
([See the dedicated Tornado engine documentation](../../engine/doc/README.md)). 

In addition the _snmptrap_ tool is required to send fake snmptrapd messages.

On Ubuntu, both the _snmptrap_ tool and the _snmptrapd_ daemon can be installed with:
```bash
sudo apt install snmp snmptrapd
```

You can now start snmptrapd (as root) in a terminal:
```bash
# snmptrapd -f -Le
```

And send fake messages with the command:
```bash
$ snmptrap -v 2c -c public localhost '' 1.3.6.1.4.1.8072.2.3.0.1 1.3.6.1.4.1.8072.2.3.2.1 i 123456
```

If everything is configured correctly, you should see a message in the snmptrapd standard error
and an Event of type _'snmptrapd'_ received by the Tornado Engine. 

In the event of authorization errors, and **_only for testing purposes_**, 
you can fix them by adding this line to the _snmptrapd.conf_ file:
```
disableAuthorization yes
```
