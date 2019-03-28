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


