/// File types
pub const T_DIR: i16 = 1; // Directory
pub const T_FILE: i16 = 2; // File
pub const T_DEVICE: i16 = 3; // Device

/// File system stat structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Stat {
    pub dev: i32,   // File system's disk device
    pub ino: u32,   // Inode number
    pub typ: i16,   // Type of file
    pub nlink: i16, // Number of links to file
    pub size: u64,  // Size of file in bytes
}
