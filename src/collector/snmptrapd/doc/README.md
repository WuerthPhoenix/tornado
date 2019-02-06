# SNMP Trap Daemon Collector

This is a collector that receives *snmptrapd* input messages formatted as JSON and generates 
an internal Event structure from them.



## How It Works

The snmptrapd input should be in the form:
```json
{
  "PDUInfo": {
    "notificationtype": "TRAP",
    "receivedfrom": "UDP: [10.62.5.31]:161->[10.62.5.115]:162",
    "version": "1",
    "errorstatus": "0",
    "messageid": "0",
    "community": "mycommunity",
    "transactionid": "1",
    "errorindex": "0",
    "requestid": "1590637175"
  },
  "VarBinds": {
    "IF-MIB::ifDescr": "4",
    "IF-MIB::ifAdminStatus.1": "2",
    "DISMAN-EVENT-MIB::sysUpTimeInstance": "67",
    "IF-MIB::ifIndex.1": "2",
    "SNMPv2-MIB::snmpTrapOID.0": "6",
    "IF-MIB::ifOperStatus.1": "2"
  }
}
```

From that input, this collector will produce the following Event:
```json
{
  "type": "snmptrapd",
  "created_ts": "2018-11-28T21:45:59.324310806+09:00",
  "payload":{
    "protocol": "UDP",
    "src_ip":"10.62.5.31",
    "src_port":"161",
    "dest_ip":"10.62.5.115",
    "oids": {
      "IF-MIB::ifDescr": "4",
      "IF-MIB::ifAdminStatus.1": "2",
      "DISMAN-EVENT-MIB::sysUpTimeInstance": "67",
      "IF-MIB::ifIndex.1": "2",
      "SNMPv2-MIB::snmpTrapOID.0": "6",
      "IF-MIB::ifOperStatus.1": "2"
    }
  }
}
``` 

The structure of the generated Event is not configurable.

As a more dynamic and configurable alternative, you can use the
[JMESPath collector](../jmespath/doc/README.md)
instead of this one.
