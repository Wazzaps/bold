#!/usr/bin/env python3
import subprocess as sp

import sys

res = sys.stdin.read()

for line in res.split('\n'):
    addr = line.strip().rpartition(' ')[2]
    print('-----------------------------------------------------')
    print(line)
    print(sp.check_output([
        'addr2line', '--functions', '--inlines', '--pretty-print', '--demangle=rust',
        '-e', 'target/aarch64-none-elf/release/boldos', addr
    ], stderr=sp.STDOUT).decode())
