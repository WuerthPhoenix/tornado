#!/usr/bin/perl
use warnings;
use strict;

use Data::Dumper;
use DateTime;
use JSON;
use JSON::XS;
use IO::Socket::INET;

# auto-flush on socket
$| = 1;

my $socket;

sub my_receiver {
    print "********** Snmptrapd_collector received a notification:\n";
    my $PDUInfo = $_[0];
    my $VarBinds = $_[1]; # Array of NetSNMP::OID

    # printTrapInfo($PDUInfo, $VarBinds);

    if (!isSocketConnected()) {
        print "Open TCP socket connection to Tornado server\n";
        $socket = IO::Socket::INET->new (
            PeerHost => '127.0.0.1',
            PeerPort => '4747',
            Proto => 'tcp',
        );
    }

    my %VarBindData;
    for (@{$VarBinds}) {
        $VarBindData{sprintf("%s",$_->[0])} = sprintf("%s", $_->[2]);
    }

    my $protocol;
    my $src_ip;
    my $src_port;
    my $dest_ip;
    my $receivedfrom = $PDUInfo->{receivedfrom};
    # print "Notification received from: $receivedfrom\n";
    if ($receivedfrom =~ m/^([^:]+):\s\[([^\]]+)\]:([0-9]+)->\[([^\]]+)\]/) {
        $protocol = $1;
        $src_ip = $2;
        $src_port = $3;
        $dest_ip = $4;
        # print "from regex: $protocol - $src_ip - $src_port - $dest_ip\n";
    };

    my $data = { 
        "type" => "snmptrapd",
        "created_ts" => getCurrentDate(),
        "payload" => {
            "protocol" => $protocol,
            "src_ip" => $src_ip,
            "src_port" => $src_port,
            "dest_ip" => $dest_ip,
            "PDUInfo" => $PDUInfo,
            "oids" => \%VarBindData,
        },
    };

    my $json = encode_json($data) . "\n";
    print $json;
    {
        local $@;
        eval{$socket->send($json);};
        print $@ if $@;
    }

    # We should return NETSNMPTRAPD_HANDLER_OK but this does not work in strict mode.
    # return NETSNMPTRAPD_HANDLER_OK;
    return 1;
}

sub isSocketConnected {
    return unless defined $socket;
    return unless $socket->connected;
    return 1;
}

sub getCurrentDate {
    my $now = DateTime->now()->iso8601().'Z';
    # my $now = DateTime->now()->format_cldr("yyyy-MM-dd'T'HH:mm:ssZ");
    return $now;
}

sub printTrapInfo {
    my ( $PDUInfo, $VarBinds ) = @_;
    
    # print the PDU info (a hash reference)
    print "PDU INFO:\n";
    foreach my $k(keys(%{$PDUInfo})) {
      if ($k eq "securityEngineID" || $k eq "contextEngineID") {
        printf "  %-30s 0x%s\n", $k, unpack('h*', $PDUInfo->{$k});
      }
      else {
        printf "  %-30s %s\n", $k, $PDUInfo->{$k};
      }
    }
 
    # print the variable bindings:
    print "VARBINDS:\n";
    foreach my $x (@{$VarBinds}) { 
        printf "  %-30s type=%-2d value=%s\n", $x->[0], $x->[2], $x->[1]; 
    }
}

NetSNMP::TrapReceiver::register("all", \&my_receiver) ||
  warn "Failed to register the perl snmptrapd_collector\n";

print STDERR "The snmptrapd_collector was loaded successfully.\n";