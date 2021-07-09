#!/usr/bin/env python3

esr = int(input(), 16)

exception_class = (esr >> 26) & 0b111111
instruction_fault_status_code = esr & 0b111111

print('exception_class:', bin(exception_class))
print('instruction_fault_status_code:', bin(instruction_fault_status_code))
