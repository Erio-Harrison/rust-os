//! On-disk file system format.
//! Both the kernel and user programs use these type definitions.

#![allow(dead_code)]

use core::mem::size_of;

/// Root inode number
pub const ROOTINO: u32 = 1;

/// Block size in bytes
pub const BSIZE: usize = 1024;

/// Magic number for filesystem identification
pub const FSMAGIC: u32 = 0x10203040;

/// Number of direct block addresses in inode
pub const NDIRECT: usize = 12;

/// Number of indirect block addresses (BSIZE / size of uint)
pub const NINDIRECT: usize = BSIZE / size_of::<u32>();

/// Maximum file size in blocks (direct + indirect)
pub const MAXFILE: usize = NDIRECT + NINDIRECT;

/// Maximum length of directory name
pub const DIRSIZ: usize = 14;

/// Disk layout:
/// [ boot block | super block | log | inode blocks |
///                                   free bit map | data blocks]
///
/// mkfs computes the super block and builds an initial file system.
/// The super block describes the disk layout.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    /// Must be FSMAGIC
    pub magic: u32,
    /// Size of file system image (blocks)
    pub size: u32,
    /// Number of data blocks
    pub nblocks: u32,
    /// Number of inodes
    pub ninodes: u32,
    /// Number of log blocks
    pub nlog: u32,
    /// Block number of first log block
    pub logstart: u32,
    /// Block number of first inode block
    pub inodestart: u32,
    /// Block number of first free map block
    pub bmapstart: u32,
}

/// On-disk inode structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DiskInode {
    /// File type
    pub type_: i16,
    /// Major device number (T_DEVICE only)
    pub major: i16,
    /// Minor device number (T_DEVICE only)
    pub minor: i16,
    /// Number of links to inode in file system
    pub nlink: i16,
    /// Size of file (bytes)
    pub size: u32,
    /// Data block addresses (NDIRECT + 1 for indirect)
    pub addrs: [u32; NDIRECT + 1],
}

/// Directory entry structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dirent {
    /// Inode number
    pub inum: u16,
    /// File name
    pub name: [u8; DIRSIZ],
}

/// Computed constants and helper functions
impl Superblock {
    /// Calculate inodes per block
    pub const IPB: usize = BSIZE / size_of::<DiskInode>();

    /// Get block containing inode i
    pub fn iblock(&self, i: u32) -> u32 {
        (i / Self::IPB as u32) + self.inodestart
    }
}

/// Bitmap bits per block
pub const BPB: usize = BSIZE * 8;

/// Get block of free map containing bit for block b
#[inline]
pub fn bblock(b: u32, sb: &Superblock) -> u32 {
    (b / BPB as u32) + sb.bmapstart
}

/// File type constants
pub mod filetype {
    pub const T_DIR: i16 = 1;     // Directory
    pub const T_FILE: i16 = 2;    // File
    pub const T_DEVICE: i16 = 3;  // Device
}