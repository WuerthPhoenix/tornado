#!/usr/bin/perl
use Data::Dumper;
use DateTime;
use JSON;
use JSON::XS;
use IO::Socket::INET;

# auto-flush on socket
$| = 1;

sub my_receiver {
    print "********** Snmptrapd_collector received a notification:\n";
    my $PDUInfo = $_[0];
    my $VarBinds = $_[1]; # Array of NetSNMP::OID

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

    my $data = {
        "type" => "snmptrapd",
        "created_ts" => getCurrentDate(),
        "payload" => {
            "PDUInfo" => $PDUInfo,
            "VarBinds" => \%VarBindData,
        },
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

## create a connecting socket
my $socket;

#die "cannot connect to the server $!\n" unless $socket;

NetSNMP::TrapReceiver::register("all", \&my_receiver) ||
  warn "Failed to register the perl snmptrapd_collector\n";

print STDERR "Loaded the perl snmptrapd_collector\n";