//! ELF (Executable and Linkable Format) type definitions

/// ELF Magic number: "\x7FELF" in little endian
pub const ELF_MAGIC: u32 = 0x464C457F;

/// ELF file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    /// Must equal ELF_MAGIC
    pub magic: u32,
    /// ELF identification
    pub elf: [u8; 12],
    /// Object file type
    pub type_: u16,
    /// Machine type
    pub machine: u16,
    /// Object file version
    pub version: u32,
    /// Entry point virtual address
    pub entry: u64,
    /// Program header table file offset
    pub phoff: u64,
    /// Section header table file offset
    pub shoff: u64,
    /// Processor-specific flags
    pub flags: u32,
    /// ELF header size in bytes
    pub ehsize: u16,
    /// Program header table entry size
    pub phentsize: u16,
    /// Program header table entry count
    pub phnum: u16,
    /// Section header table entry size
    pub shentsize: u16,
    /// Section header table entry count
    pub shnum: u16,
    /// Section header string table index
    pub shstrndx: u16,
}

/// Program section header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    /// Segment type
    pub type_: u32,
    /// Segment flags
    pub flags: u32,
    /// Segment file offset
    pub off: u64,
    /// Segment virtual address
    pub vaddr: u64,
    /// Segment physical address
    pub paddr: u64,
    /// Segment size in file
    pub filesz: u64,
    /// Segment size in memory
    pub memsz: u64,
    /// Segment alignment
    pub align: u64,
}

/// Program header type values
pub mod ph_type {
    /// Loadable program segment
    pub const LOAD: u32 = 1;
}

/// Program header flags
pub mod ph_flags {
    /// Execute permission
    pub const EXEC: u32 = 1;
    /// Write permission
    pub const WRITE: u32 = 2;
    /// Read permission
    pub const READ: u32 = 4;
}

impl ElfHeader {
    /// Check if the header has a valid ELF magic number
    pub fn is_valid(&self) -> bool {
        self.magic == ELF_MAGIC
    }
}
