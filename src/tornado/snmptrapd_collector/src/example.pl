#!/usr/bin/perl
 
sub my_receiver {
    print "********** PERL RECEIVED A NOTIFICATION:\n";
 
    # print the PDU info (a hash reference)
    print "PDU INFO:\n";
    foreach my $k(keys(%{$_[0]})) {
      if ($k eq "securityEngineID" || $k eq "contextEngineID") {
        printf "  %-30s 0x%s\n", $k, unpack('h*', $_[0]{$k});
      }
      else {
        printf "  %-30s %s\n", $k, $_[0]{$k};
      }
    }
 
    # print the variable bindings:
    print "VARBINDS:\n";
    foreach my $x (@{$_[1]}) { 
        printf "  %-30s type=%-2d value=%s\n", $x->[0], $x->[2], $x->[1]; 
    }
}
 
NetSNMP::TrapReceiver::register("all", \&my_receiver) || 
  warn "failed to register our perl trap handler\n";
 
print STDERR "Loaded the example perl snmptrapd handler\n";
