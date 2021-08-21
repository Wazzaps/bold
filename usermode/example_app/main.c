#include <stddef.h>

__attribute__((always_inline))
static inline long __syscall0(long syscall_no) {
    register unsigned long r8 __asm("x8") = syscall_no;
    register unsigned long r0 __asm("x0") = 0;
    __asm__ __volatile__ ("svc #0" : "+r"(r0) : "r"(r8) : "memory");
    return r0;
}

__attribute__((always_inline))
static inline long __syscall1(long syscall_no, unsigned long arg1) {
    register unsigned long r8 __asm("x8") = syscall_no;
    register unsigned long r0 __asm("x0") = arg1;
    __asm__ __volatile__ ("svc #0" : "+r"(r0) : "r"(r8), "r"(r0) : "memory");
    return r0;
}

#define SYS_EXIT 0
#define SYS_KLOG_WRITE 1
#define SYS_KLOG_WRITE_INT 2
#define SYS_USLEEP 3
#define SYS_GET_TID 4


void _start() {
    size_t my_tid = __syscall0(SYS_GET_TID);
    __syscall1(SYS_KLOG_WRITE, (unsigned long) "Hello from usermode! &start =");
    __syscall1(SYS_KLOG_WRITE_INT, (unsigned long) &_start);

    // __syscall1(SYS_KLOG_WRITE, (unsigned long) "Trying to read kernel memory:");
    // __syscall1(SYS_KLOG_WRITE_INT, *(unsigned long*) 0xffffff8000080000);

    for (size_t i = 0; i < 50; i++) {
        __syscall1(SYS_KLOG_WRITE, (unsigned long) "Sleeping for 1 sec, my_tid =");
        __syscall1(SYS_KLOG_WRITE_INT, (unsigned long) my_tid);
        __syscall1(SYS_USLEEP, (unsigned long) 1000000);
    }

    __syscall1(SYS_KLOG_WRITE, (unsigned long) "Bye!");
    __syscall1(SYS_EXIT, (unsigned long) 0);
    __syscall1(SYS_KLOG_WRITE, (unsigned long) "If you see this then I survived an exit :(");
    __syscall1(SYS_EXIT, (unsigned long) 0);
}
