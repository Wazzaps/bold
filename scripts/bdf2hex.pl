#!/usr/bin/perl -n

if (/^ENCODING\s+(\d+)/) {  }
elsif (/^BITMAP/) { $BITMAP=1; }
elsif (/^ENDCHAR/) { $BITMAP=0; print "\n"; }
elsif ($BITMAP) { y/a-f/A-F/; s/\n$//; print; }

