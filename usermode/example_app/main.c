__attribute__((always_inline))
static inline long __syscall0(long syscall_no) {
    register unsigned long r0 __asm("x0") = syscall_no;
    __asm__ __volatile__ ("svc #0" : "+r"(r0) : "r"(r0) : "memory");
    return r0;
}

__attribute__((always_inline))
static inline long __syscall1(long syscall_no, unsigned long arg1) {
    register unsigned long r0 __asm("x0") = syscall_no;
    register unsigned long r1 __asm("x1") = arg1;
    __asm__ __volatile__ ("svc #0" : "+r"(r0) : "r"(r0), "r"(r1) : "memory");
    return r0;
}

#define SYS_EXIT 0
#define SYS_KLOG_WRITE 1
#define SYS_KLOG_WRITE_INT 2


void _start() {
    __syscall1(SYS_KLOG_WRITE, (unsigned long) "Hello from usermode! &start =");
    __syscall1(SYS_KLOG_WRITE_INT, (unsigned long) &_start);
    __syscall1(SYS_KLOG_WRITE, (unsigned long) "Bye!");
//    __syscall1(SYS_KLOG_WRITE, (unsigned long) "Trying to read kernel memory:");
//    __syscall1(SYS_KLOG_WRITE_INT, *(unsigned long*) 0xffffff8000080000);
    __syscall1(SYS_EXIT, (unsigned long) 0);
}
