# SNMP Trap Daemon Collector

This is a collector that receives *snmptrapd* input messages formatted as JSON and generates
an internal Event structure from them.



## How It Works

The snmptrapd input should be in the form:
```
PDU INFO:
  version                        1
  errorstatus                    0
  community                      public
  receivedfrom                   UDP: [127.0.0.1]:41543->[127.0.0.1]:162
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

From that input, this collector will produce the following Event:
```json
{
   "type":"snmptrapd",
   "created_ts":"2019-03-28T09:38:10Z",
   "payload":{
      "protocol":"UDP",
      "dest_ip":"127.0.0.1",
      "src_port":"41543",
      "src_ip":"127.0.0.1",
      "PDUInfo":{
         "version":"1",
         "errorstatus":"0",
         "community":"public",
         "receivedfrom":"UDP: [127.0.0.1]:41543->[127.0.0.1]:162",
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

# Install snmptrapd

sudo apt install snmp snmptrapd


#Requirements:

- Perl 5.16
- Perl packages:
-- DateTime
-- JSON
-- NetSNMP

To install perl packages:
- Ubuntu:
> sudo apt install libdatetime-perl libjson-perl libsnmp-perl

To check that libs are available:
> perl -e 'use JSON;' && perl -e 'use NetSNMP::TrapReceiver;'

if you should see no messages in the console, then everything's ok.


The embedded snmptrapd perl script should automatically reconnect in case the Tornado engine is stopped and restarted.


# Configure snmptrapd

Bind a perl script to an snmptrapd event:
- edit the file: /etc/snmp/snmptrapd.conf
- add the line: perl do "path_to_your_script/script.pl";

When a trap is received, if you see this error:
`No access configuration - dropping trap.`
Add to the snmptrapd.conf file this line:
`disableAuthorization yes`


# send a fake snmp

Start snmptrapd (as root, and the following other opions make it stay in the foreground and log to stderr):
> snmptrapd -f -Le

send an snmp message
> snmptrap -v 2c -c public localhost '' 1.3.6.1.4.1.8072.2.3.0.1 1.3.6.1.4.1.8072.2.3.2.1 i 123456


