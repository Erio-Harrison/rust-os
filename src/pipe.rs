use crate::param::PIPESIZE;

use super::spinlock::SpinLock;

#[repr(C)]
pub struct Pipe {
    pub lock: SpinLock,
    pub data: [u8; PIPESIZE],
    pub nread: u32,
    pub nwrite: u32,
    pub readopen: bool,
    pub writeopen: bool,
}