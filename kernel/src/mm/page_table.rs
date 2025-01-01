use bitflags::bitflags;

bitflags! {
    pub struct PageTableFlags: u8 {
        const VALID =     1 << 0;
        const READABLE =  1 << 1;
        const WRITABLE =  1 << 2;
        const EXECUTABLE = 1 << 3;
        const USER =      1 << 4;
        const GLOBAL =    1 << 5;
        const ACCESSED =  1 << 6;
        const DIRTY =     1 << 7;
    }
}

#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageTableEntry {
    bits: usize,
}

impl PageTableEntry {
    pub fn new(addr: usize, flags: PageTableFlags) -> Self {
        PageTableEntry {
            bits: (addr >> 12 << 10) | flags.bits() as usize,
        }
    }
    
    pub fn addr(&self) -> usize {
        (self.bits >> 10) << 12
    }
    
    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.bits as u8)
    }
}