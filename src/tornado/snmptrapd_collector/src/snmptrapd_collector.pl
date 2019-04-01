#!/usr/bin/perl
use warnings;
use strict;

use Data::Dumper;
use DateTime;
use Cpanel::JSON::XS;
use IO::Socket::INET;
use NetSNMP::TrapReceiver qw/NETSNMPTRAPD_HANDLER_OK/;
use threads;
use threads::shared;
use Thread::Queue;

# auto-flush on socket
$| = 1;

my $socket;

my $sleep_seconds_between_connection_attempts = 5;

my $max_events_queue_size = 10000;
my $events_queue = Thread::Queue->new();

my $tornado_writer = async {
	eval {
		print "[tornado_writer] started\n";
		my $json_event;
		while ($json_event = $events_queue->dequeue()) {

		    # print "[tornado_writer] received event:\n$json_event\n";
		    if (!isSocketConnected()) {
                my $ip = getEnvOrDefault("TORNADO_ADDR", "127.0.0.1");
                my $port = getEnvOrDefault("TORNADO_PORT", "4747");

                print "Open TCP socket connection to Tornado server at $ip:$port\n";
                $socket = IO::Socket::INET->new (
                    PeerHost => $ip,
                    PeerPort => $port,
                    Proto => 'tcp',
                );
            }

		    {
                local $@;
                eval{$socket->send($json_event);};

                # The $socket->send($json_event) executes transparently even if the event is not sent because the connection to Tornado is dropped.
                # The only way we found to check whether the send was performed correctly, is to verify that the socket is still connected after a send.
                # In fact, it will return 0 in case of issues during the send.
                my $failed = !isSocketConnected();

                # This 'if' condition is true when the $socket variable is undefined.
                # This happens when the socket cannot be created because Tornado is not available.
                if ($@) {
                    # print "[tornado_writer] cannot send Event to Tornado Server: $@\n";
                    $failed = 1;
                }

                if ($failed) {
                    print "[tornado_writer] cannot send Event to Tornado Server! Attempt a new connection in $sleep_seconds_between_connection_attempts seconds\n";
                    enqueue($json_event);
                    sleep($sleep_seconds_between_connection_attempts);
                }
            }

		}
		print "[tornado_writer] stopped\n";
	};
	if ($@) {
		print "[tornado_writer] FATAL: $@\n";
	}
};

sub my_receiver {
    # print "********** Snmptrapd_collector received a notification:\n";
    my $PDUInfo = $_[0];
    my $VarBinds = $_[1]; # Array of NetSNMP::OID

    # printTrapInfo($PDUInfo, $VarBinds);

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
        "created_ms" => getCurrentDate(),
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
    # print $json;
    # push it in the queue
    enqueue($json);

    return NETSNMPTRAPD_HANDLER_OK;
}

sub enqueue {
    my ( $data ) = @_;
    if ($events_queue->pending() < $max_events_queue_size ) {
        $events_queue->enqueue($data);
    } else {
        print "WARN: The event buffer is full (max allowed: $max_events_queue_size events). New messages will be discarded!!"
    }
}

sub isSocketConnected {
    return 0 unless defined $socket;
    return 0 unless $socket->connected;
    return 1;
}

sub getCurrentDate {
    my $now = DateTime->now()->iso8601().'Z';
    # my $now = DateTime->now()->format_cldr("yyyy-MM-dd'T'HH:mm:ssZ");
    return $now;
}

sub getEnvOrDefault {
    my ( $key, $default ) = @_;
    my $envValue = $ENV{$key};
    # print "KEY is $key - VALUE is $envValue\n";
    if ($envValue) {
        return $envValue;
    }
    return $default;
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