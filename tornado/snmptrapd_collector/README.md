# SNMP Trap Daemon Collectors

The _snmptrapd_collector_s of this package are embedded Perl trap handlers for Net-SNMP's snmptrapd.
When registered as a subroutine in the Net-SNMP snmptrapd process, they receives
snmptrap-specific inputs, transforms them into Tornado Events, and forwards them to
the Tornado Engine.

There are two collector implementations, the first one sends Events directly to 
the Tornado TCP socket and the second one forwards them to a NATS server.

The implementations rely on the Perl NetSNMP::TrapReceiver package. You can refer to
[its documentation](https://metacpan.org/pod/NetSNMP::TrapReceiver)
for generic configuration examples and usage advice. 


## SNMPTrapd TCP Collector Configuration

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
perl do "/path_to_the_script/snmptrapd_tcp_collector.pl"; 
```

Consequently, it is never started manually, but instead will be started, and managed,
directly by _snmptrapd_ itself.

At startup, if the collector is configured properly, you should see 
this entry either in the logs or in the daemon's standard error output:
```
The TCP based snmptrapd_collector was loaded successfully.
```


### Configuration options

The address of the Tornado Engine TCP instance to which the events are forwarded 
is configured with the following environment variables:
- __TORNADO_ADDR__: the IP address of Tornado Engine. If not specified, 
  it will use the default value _127.0.0.1_
- __TORNADO_PORT__: the port of the TCP socket of Tornado Engine. If not specified, 
  it will use the default value _4747_


## SNMPTrapd NATS Collector Configuration

### Prerequisites

This collector has the following runtime requirements:
- Perl 5.16 or greater
- Perl packages required:
  - Cpanel::JSON::XS
  - Net::NATS::Client
  - NetSNMP::TrapReceiver

You can verify that the Perl packages are available with the command:
```bash
$ perl -e 'use Cpanel::JSON::XS;' && \
  perl -e 'use Net::NATS::Client;' && \
  perl -e 'use NetSNMP::TrapReceiver;'
```

If no messages are displayed in the console, then everything is okay; otherwise, 
you will see error messages.

In case of missing dependencies, use your system's package manager to install them.

Instructions for installing `Net::NATS::Client` are available at 
its [official repository](https://github.com/carwynmoore/perl-nats)


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
The snmptrapd_collector for NATS was loaded successfully.
```


### Configuration options

The information to connect to the NATS Server are provided by the following environment variables:
- __TORNADO_NATS_ADDR__: the address of the NATS server. If not specified, 
  it will use the default value _127.0.0.1:4222_
- __TORNADO_NATS_SUBJECT__: the NATS subject where the events are published. If not specified, 
  it will use the default value _tornado.events_
- __TORNADO_NATS_SSL_CERT_PEM_FILE__: The filesystem path of a PEM certificate. 
This entry is optional, when provided, the collector will use the certificate to connect to the NATS server   
- __TORNADO_NATS_SSL_CERT_KEY__: The filesystem path for the KEY of the PEM certificate provided by the
*TORNADO_NATS_SSL_CERT_PEM_FILE* entry. This entry is mandatory if the *TORNADO_NATS_SSL_CERT_PEM_FILE* entry
is provided


## How They Work

The _snmptrapd_collector_s receive snmptrapd messages, parse them, generate Tornado Events
and, finally, sends them to Tornado using their specific communication channel.

The received messages are kept in an in-memory non-persistent buffer that makes the application
resilient to crashes or temporary unavailability of the communication channel. 
When the connection to the channel is restored, all
messages in the buffer will be sent.  When the buffer is full, the collectors will start
discarding old messages. 
The buffer max size is set to `10000` messages. 
 
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



## Testing

To test the collector, verify that snmptrapd is installed on the machine and
follow the collector configuration instructions above.

As a prerequisite, the Tornado Engine should be up and running on the same machine 
([See the dedicated Tornado engine documentation](../engine/README.md)). 

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
you can fix them by adding this line to the _snmptrapd.conf_ file (in Ubuntu you can find it in */etc/snmp/snmptrapd.conf*):
```
disableAuthorization yes
```



## Extending MIBs

SNMP relies on MIB (Management Information Base) definition files, but the *net-snmp* toolkit
used in NetEye does not come with a complete set for all network devices.  You may thus find
it necessary to add new definitions when configuring Tornado in your environment.

If you have not previously set up *net-snmp* tools, you can enable the principle command as
follows:
```
yum install /usr/bin/snmptranslate
```

If your device is already in the system, this command will return its OID, or else an error:
```
# snmptranslate -IR -On snmpTrapOID
.1.3.6.1.6.3.1.1.4.1
# snmptranslate -IR -On ciscoLS1010ChassisFanLed
Unknown object identifier: ciscoLS1010ChassisFanLed
```

If your device is not known, you can download its MIB file (e.g., from
[Cisco](ftp://ftp.cisco.com/pub/mibs/v2/)) and place it in the default NetEye directory:
```
/usr/share/snmp/mibs
```

You will then need to make *net-snmp* aware of the new configuration and ensure it is reloaded
automatically on reboot.  More information can be found at the
[official Net-SNMP website](http://net-snmp.sourceforge.net/wiki/index.php/TUT:Using_and_loading_MIBS).
