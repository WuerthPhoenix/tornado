#!/usr/bin/perl

# Work In Progress
# TODO UDS Socket
# Reconnect / Resilience

use strict;
use warnings;

use Data::Dumper;
use JSON;
use JSON::XS;
use IO::Socket::INET;

# auto-flush on socket
$| = 1;

sub my_receiver {
    print "********** PERL RECEIVED A NOTIFICATION $counter:\n";
    my $PDUInfo = $_[0];
    my $VarBinds = $_[1]; # Array of NetSNMP::OID

    if (!$socket) {
        print "CRATING NEW SOCKET!";
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

    my $data = {
        "counter" => $counter++,
        "PDUInfo" => $PDUInfo,
        "VarBinds" => \%VarBindData,
    };

    my $json = encode_json($data) . "\n";
    print $json;
    {
        local $@;
        eval{$socket->send($json);};
        print $@ if $@;
    }
    return 1;
}

my $counter = 1;
## create a connecting socket
my $socket = IO::Socket::INET->new (
    PeerHost => '127.0.0.1',
    PeerPort => '4747',
    Proto => 'tcp',
);

#die "cannot connect to the server $!\n" unless $socket;
print "connected to the server\n";


NetSNMP::TrapReceiver::register("all", \&my_receiver) ||
    warn "failed to register our perl trap handler\n";

print STDERR "Loaded the example perl snmptrapd handler\n";
