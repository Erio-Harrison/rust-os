use super::spinlock::SpinLock;
use super::param::*;

#[derive(Copy, Clone, PartialEq)]
pub enum FileType {
    FD_NONE,
    FD_PIPE,
    FD_INODE,
    FD_DEVICE,
}

#[repr(C)]
pub struct File {
    pub typ: FileType,
    pub ref_count: i32,
    pub readable: bool,
    pub writable: bool,
    pub pipe: *mut Pipe,
    pub ip: *mut Inode,
    pub off: u32,
    pub major: i16,
}

#[repr(C)]
pub struct Pipe {
    pub lock: SpinLock,
    pub data: [u8; PIPESIZE],
    pub nread: u32,
    pub nwrite: u32,
    pub readopen: bool,
    pub writeopen: bool,
}

#[repr(C)]
pub struct Inode {
    pub dev: u32,           // 设备号
    pub inum: u32,          // Inode号
    pub ref_count: i32,     // 引用计数
    pub lock: SleepLock,    // 保护以下所有字段
    pub valid: bool,        // inode是否已从磁盘读取
    pub typ: i16,          // 从磁盘inode复制
    pub major: i16,
    pub minor: i16,
    pub nlink: i16,
    pub size: u32,
    pub addrs: [u32; NDIRECT+1],
}

// TODO: 需要定义 SleepLock
#[repr(C)]
pub struct SleepLock {
    // 需要实现
    pub locked: bool,
    pub lock: SpinLock,
}