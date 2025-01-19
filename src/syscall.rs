use crate::{println, proc::myproc, string::strlen, vm::{copyin, copyinstr}};

// System call numbers
pub const SYS_FORK: usize = 1;
pub const SYS_EXIT: usize = 2;
pub const SYS_WAIT: usize = 3;
pub const SYS_PIPE: usize = 4;
pub const SYS_READ: usize = 5;
pub const SYS_KILL: usize = 6;
pub const SYS_EXEC: usize = 7;
pub const SYS_FSTAT: usize = 8;
pub const SYS_CHDIR: usize = 9;
pub const SYS_DUP: usize = 10;
pub const SYS_GETPID: usize = 11;
pub const SYS_SBRK: usize = 12;
pub const SYS_SLEEP: usize = 13;
pub const SYS_UPTIME: usize = 14;
pub const SYS_OPEN: usize = 15;
pub const SYS_WRITE: usize = 16;
pub const SYS_MKNOD: usize = 17;
pub const SYS_UNLINK: usize = 18;
pub const SYS_LINK: usize = 19;
pub const SYS_MKDIR: usize = 20;
pub const SYS_CLOSE: usize = 21;

pub const NSYSCALL: usize = 22;

// System call helper functions

/// Fetch the u64 at `addr` from the current process.
pub unsafe fn fetchaddr(addr: u64, ip: *mut u64) -> i32 {
    let p = myproc();

    // Check if the address is within the process's memory space
    if addr >= (*p).sz || addr + core::mem::size_of::<u64>() as u64 > (*p).sz {
        return -1;  // Address out of bounds
    }
    
    // Copy the data from user space to kernel space
    if copyin(
        (*p).pagetable as *mut usize,
        ip as *mut u8,
        addr,
        core::mem::size_of::<u64>() as u64,
    ) != 0 {
        return -1;  // Copy failed
    }
    
    0  // Success
}

/// Fetch the null-terminated string at `addr` from the current process.
/// Returns the length of the string, not including the null terminator, or -1 for error.
pub unsafe fn fetchstr(addr: u64, buf: *mut u8, max: i32) -> i32 {
    let p = myproc();
    
    // Copy the string from user space to kernel space
    if copyinstr((*p).pagetable as *mut usize, buf, addr, max as u64) < 0 {
        return -1;  // Copy failed
    }
    
    // Return the length of the string
    strlen(buf) as i32
}

/// Retrieve the raw system call argument at index `n`.
unsafe fn argraw(n: i32) -> u64 {
    let p = myproc();
    
    // Fetch the argument from the trapframe based on the index
    match n {
        0 => (*(*p).trapframe).a0,
        1 => (*(*p).trapframe).a1,
        2 => (*(*p).trapframe).a2,
        3 => (*(*p).trapframe).a3,
        4 => (*(*p).trapframe).a4,
        5 => (*(*p).trapframe).a5,
        _ => panic!("argraw"),  // Invalid argument index
    }
}

/// Fetch the nth 32-bit system call argument.
pub unsafe fn argint(n: i32, ip: *mut i32) {
    *ip = argraw(n) as i32;
}

/// Retrieve an argument as a pointer.
pub unsafe fn argaddr(n: i32, ip: *mut u64) {
    *ip = argraw(n);
}

/// Fetch the nth word-sized system call argument as a null-terminated string.
pub unsafe fn argstr(n: i32, buf: *mut u8, max: i32) -> i32 {
    let mut addr: u64 = 0;
    argaddr(n, &mut addr);  // Get the address of the string
    fetchstr(addr, buf, max)  // Fetch the string from user space
}

// Declare system call functions
extern "C" {
    fn sys_fork() -> u64;
    fn sys_exit() -> u64;
    fn sys_wait() -> u64;
    fn sys_pipe() -> u64;
    fn sys_read() -> u64;
    fn sys_kill() -> u64;
    fn sys_exec() -> u64;
    fn sys_fstat() -> u64;
    fn sys_chdir() -> u64;
    fn sys_dup() -> u64;
    fn sys_getpid() -> u64;
    fn sys_sbrk() -> u64;
    fn sys_sleep() -> u64;
    fn sys_uptime() -> u64;
    fn sys_open() -> u64;
    fn sys_write() -> u64;
    fn sys_mknod() -> u64;
    fn sys_unlink() -> u64;
    fn sys_link() -> u64;
    fn sys_mkdir() -> u64;
    fn sys_close() -> u64;
}

// System call table
type SyscallFn = unsafe extern "C" fn() -> u64;

static mut SYSCALLS: [Option<SyscallFn>; NSYSCALL] = [None; NSYSCALL];

// Initialize the system call table
pub unsafe fn syscall_init() {
    SYSCALLS[SYS_FORK] = Some(sys_fork);
    SYSCALLS[SYS_EXIT] = Some(sys_exit);
    SYSCALLS[SYS_WAIT] = Some(sys_wait);
    SYSCALLS[SYS_PIPE] = Some(sys_pipe);
    SYSCALLS[SYS_READ] = Some(sys_read);
    SYSCALLS[SYS_KILL] = Some(sys_kill);
    SYSCALLS[SYS_EXEC] = Some(sys_exec);
    SYSCALLS[SYS_FSTAT] = Some(sys_fstat);
    SYSCALLS[SYS_CHDIR] = Some(sys_chdir);
    SYSCALLS[SYS_DUP] = Some(sys_dup);
    SYSCALLS[SYS_GETPID] = Some(sys_getpid);
    SYSCALLS[SYS_SBRK] = Some(sys_sbrk);
    SYSCALLS[SYS_SLEEP] = Some(sys_sleep);
    SYSCALLS[SYS_UPTIME] = Some(sys_uptime);
    SYSCALLS[SYS_OPEN] = Some(sys_open);
    SYSCALLS[SYS_WRITE] = Some(sys_write);
    SYSCALLS[SYS_MKNOD] = Some(sys_mknod);
    SYSCALLS[SYS_UNLINK] = Some(sys_unlink);
    SYSCALLS[SYS_LINK] = Some(sys_link);
    SYSCALLS[SYS_MKDIR] = Some(sys_mkdir);
    SYSCALLS[SYS_CLOSE] = Some(sys_close);
}

/// System call handler
pub unsafe fn syscall() {
    let p = myproc();
    let num = (*(*p).trapframe).a7 as usize;  // Get the system call number

    // Check if the system call number is valid
    if num > 0 && num < SYSCALLS.len() {
        if let Some(syscall_fn) = SYSCALLS[num] {
            // Call the system call function and store the result in the trapframe
            (*(*p).trapframe).a0 = syscall_fn();
        } else {
            // Unknown system call
            println!(
                "{} {}: unknown sys call {}",
                (*p).pid,
                core::str::from_utf8_unchecked(&(*p).name),
                num
            );
            (*(*p).trapframe).a0 = -1i64 as u64;  // Return an error
        }
    } else {
        // Invalid system call number
        println!(
            "{} {}: unknown sys call {}",
            (*p).pid,
            core::str::from_utf8_unchecked(&(*p).name),
            num
        );
        (*(*p).trapframe).a0 = -1i64 as u64;  // Return an error
    }
}