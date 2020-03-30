#!/usr/bin/perl
use warnings;
use strict;

use Cpanel::JSON::XS;
use IO::Socket::INET;
use Net::NATS::Client;
use NetSNMP::TrapReceiver qw/NETSNMPTRAPD_HANDLER_OK/;
use Time::HiRes qw/gettimeofday/;
use threads;
use threads::shared;
use Thread::Queue;

# auto-flush on socket
$| = 1;

# Without this one, if the `$client->publish` call fails, the perl process is killed.
# Calling `$client->publish` into an `eval` block does not solve the issue.
$SIG{PIPE} = 'IGNORE';

my $client;

my $sleep_seconds_between_connection_attempts = 5;

my $max_events_queue_size = 10000;
my $events_queue = Thread::Queue->new();

my $subject = getEnvOrDefault("TORNADO_NATS_SUBJECT", "tornado.events");
my $tornado_writer = async {
	eval {
		print "[tornado_nats_writer] started\n";
		my $json_event;
		while ($json_event = $events_queue->dequeue()) {

		    # print "[tornado_nats_writer] received event:\n$json_event\n";
		    if (! defined $client) {
		        my $ssl_cert_file = getEnvOrDefault("TORNADO_NATS_SSL_CERT_PEM_FILE", "");
		        my $ssl_cert_key = getEnvOrDefault("TORNADO_NATS_SSL_CERT_KEY", "");
                my $addr = getEnvOrDefault("TORNADO_NATS_ADDR", "127.0.0.1:4222");
                $addr = "nats://$addr";

                if ($ssl_cert_file ne "" && $ssl_cert_key ne "") {
                    print "Start NATS connection to server $addr with SSL certificate [$ssl_cert_file] and key [$ssl_cert_key]\n";
                    my $socket_args = {
                        SSL_cert_file => $ssl_cert_file,
                        SSL_key_file => $ssl_cert_key,
                    };
                    $client = Net::NATS::Client->new(uri => $addr, socket_args => $socket_args);
                } else {
                    print "Start NATS connection to server $addr without SSL certificate\n";
                    $client = Net::NATS::Client->new(uri => $addr);
                }

                $client->connect();
                print "Connected to NATS server $addr\n";
            }

		    {
                local $@;
                my $result;
                eval{$result = $client->publish($subject, $json_event);};

                my $failed = ! defined $result || $result ne 1;

                if ($@ || $!) {
                     print "[tornado_nats_writer] Cannot send Event to the NATS Server:$!$@\n";
                     $failed = 1;
                }

                if ($failed) {
                    print "[tornado_nats_writer] Cannot send Event to the NATS Server! Attempt a new connection in $sleep_seconds_between_connection_attempts seconds\n";
                    enqueue($json_event);
                    sleep($sleep_seconds_between_connection_attempts);
                    eval{$client->close();};
                    $client = undef;
                }
            }

		}
		print "[tornado_nats_writer] stopped\n";
	};
	if ($@) {
		print "[tornado_nats_writer] FATAL: $@\n";
	}
};

sub my_receiver {
    # print "********** Snmptrapd_collector received a notification:\n";
    my $PDUInfo = $_[0];
    my $VarBinds = $_[1]; # Array of NetSNMP::OID

    # printTrapInfo($PDUInfo, $VarBinds);

    my %VarBindData;
    for (@{$VarBinds}) {
        # Data is available in the form of $DATATYPE: $CONTENT
        # We split at the first ocurrence of ': '
        my ($datatype, $content) = split /: /, $_->[1], 2;

        # Drop the escaped leading and trailing " for strings
        if ( $datatype =~ m/^string$/i ) {
            $content = substr $content, 1, -1;
        }

        # Provide both datatype in text form as well as content in JSON e.g.
        # "NMPv2-SMI::enterprises.6876.4.3.305.": {
        #     "datatype": "STRING",
        #     "content": "Yellow"
        # }
        my %oid = (
            "datatype" => $datatype,
            "content" => $content
        );
        $VarBindData{sprintf("%s",$_->[0])} = \%oid;
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
        "created_ms" => getCurrentEpochMs(),
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

sub getCurrentEpochMs {
    my $now = int (gettimeofday * 1000);
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
  warn "Failed to register the perl snmptrapd_collector for NATS\n";

print STDERR "The snmptrapd_collector for NATS was loaded successfully.\n";