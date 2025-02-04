use crate::{
    kalloc::{kalloc, kfree},
    memlayout::{KERNBASE, PHYSTOP, PLIC, TRAMPOLINE, UART0, VIRTIO0},
    proc::proc_mapstacks,
    riscv_local::{
        make_satp, pa2pte, pgrounddown, pgroundup, pte2pa, pte_flags, sfence_vma, w_satp, MAXVA,
        PGSIZE, PTE_R, PTE_U, PTE_V, PTE_W, PTE_X,
    },
};
use core::ptr;

pub type PageTable = *mut usize;
pub type PTE = u64;

static mut KERNEL_PAGETABLE: PageTable = ptr::null_mut();

// Provided by linker
extern "C" {
    static etext: usize;
    static trampoline: usize;
}

unsafe fn kvmmake() -> PageTable {
    let kpgtbl = kalloc() as PageTable;
    // Initialize page table with zeros
    ptr::write_bytes(kpgtbl, 0, PGSIZE.try_into().unwrap());

    // uart registers
    kvmmap(kpgtbl, UART0, UART0, PGSIZE, PTE_R | PTE_W);

    // virtio mmio disk interface
    kvmmap(kpgtbl, VIRTIO0, VIRTIO0, PGSIZE, PTE_R | PTE_W);

    // PLIC
    kvmmap(kpgtbl, PLIC, PLIC, 0x400_0000, PTE_R | PTE_W);

    // map kernel text executable and read-only
    kvmmap(
        kpgtbl,
        KERNBASE,
        KERNBASE,
        (&etext as *const _ as usize - KERNBASE as usize)
            .try_into()
            .unwrap(),
        (PTE_R | PTE_X).try_into().unwrap(),
    );

    // map kernel data and the physical RAM we'll make use of
    kvmmap(
        kpgtbl,
        &etext as *const _ as u64,
        &etext as *const _ as u64,
        PHYSTOP - &etext as *const _ as u64,
        (PTE_R | PTE_W).try_into().unwrap(),
    );

    // map the trampoline for trap entry/exit to
    // the highest virtual address in the kernel
    kvmmap(
        kpgtbl,
        TRAMPOLINE,
        &trampoline as *const _ as u64,
        PGSIZE,
        (PTE_R | PTE_X).try_into().unwrap(),
    );

    // allocate and map a kernel stack for each process
    proc_mapstacks(kpgtbl);

    kpgtbl
}

/// Initialize the one kernel_pagetable
pub unsafe fn kvminit() {
    KERNEL_PAGETABLE = kvmmake();
}

/// Switch h/w page table register to the kernel's page table,
/// and enable paging.
pub unsafe fn kvminithart() {
    // wait for any previous writes to the page table memory to finish
    sfence_vma();

    // set page table register
    w_satp(make_satp(KERNEL_PAGETABLE as u64).try_into().unwrap());

    // flush stale entries from the TLB
    sfence_vma();
}

/// Return the address of the PTE in page table pagetable
/// that corresponds to virtual address va.  If alloc!=0,
/// create any required page-table pages.
///
/// The risc-v Sv39 scheme has three levels of page-table
/// pages. A page-table page contains 512 64-bit PTEs.
/// A 64-bit virtual address is split into five fields:
///   39..63 -- must be zero.
///   30..38 -- 9 bits of level-2 index.
///   21..29 -- 9 bits of level-1 index.
///   12..20 -- 9 bits of level-0 index.
///    0..11 -- 12 bits of byte offset within the page.
unsafe fn walk(pagetable: PageTable, va: u64, alloc: bool) -> *mut PTE {
    if va >= MAXVA {
        panic!("walk");
    }

    const PXMASK: u64 = 0x1FF; // 9 bits
    let px = |level| ((va >> (12 + 9 * level)) & PXMASK) as usize;

    let mut pagetable = pagetable as *mut PTE;

    // for each level of the page table
    for level in (1..=2).rev() {
        let pte = &mut *pagetable.add(px(level));

        if *pte & PTE_V as u64 != 0 {
            pagetable = (((*pte) >> 10) << 12) as *mut PTE;
        } else {
            if !alloc {
                return ptr::null_mut();
            }
            let new_page = kalloc();
            if new_page.is_null() {
                return ptr::null_mut();
            }
            ptr::write_bytes(new_page, 0, PGSIZE.try_into().unwrap());
            *pte = (((new_page as u64) >> 12) << 10) | PTE_V as u64;
            pagetable = new_page as *mut PTE;
        }
    }

    pagetable.add(px(0))
}

/// Look up a virtual address, return the physical address,
/// or 0 if not mapped.
/// Can only be used to look up user pages.
pub unsafe fn walkaddr(pagetable: PageTable, va: u64) -> u64 {
    if va >= MAXVA {
        return 0;
    }

    let pte = walk(pagetable, va, false);
    if pte.is_null() {
        return 0;
    }

    let pte = *pte;
    if (pte & PTE_V as u64) == 0 || (pte & PTE_U as u64) == 0 {
        return 0;
    }

    (pte >> 10) << 12
}

/// add a mapping to the kernel page table.
/// only used when booting.
/// does not flush TLB or enable paging.
pub unsafe fn kvmmap(kpgtbl: PageTable, va: u64, pa: u64, size: u64, perm: u64) {
    if mappages(kpgtbl, va, size, pa, perm) != 0 {
        panic!("kvmmap");
    }
}

/// Create PTEs for virtual addresses starting at va that refer to
/// physical addresses starting at pa.
/// va and size MUST be page-aligned.
/// Returns 0 on success, -1 if walk() couldn't
/// allocate a needed page-table page.
pub unsafe fn mappages(pagetable: PageTable, va: u64, size: u64, pa: u64, perm: u64) -> isize {
    if va % PGSIZE != 0 {
        panic!("mappages: va not aligned");
    }

    if size % PGSIZE != 0 {
        panic!("mappages: size not aligned");
    }

    if size == 0 {
        panic!("mappages: size");
    }

    let last = va + size - PGSIZE;
    let mut a = va;
    let mut pa = pa;

    loop {
        let pte = walk(pagetable, a, true);
        if pte.is_null() {
            return -1;
        }
        if *pte & PTE_V != 0 {
            panic!("mappages: remap");
        }
        *pte = pa2pte(pa) | perm | PTE_V;
        if a == last {
            break;
        }
        a += PGSIZE;
        pa += PGSIZE;
    }
    0
}

/// Remove npages of mappings starting from va. va must be
/// page-aligned. The mappings must exist.
/// Optionally free the physical memory.
pub unsafe fn uvmunmap(pagetable: PageTable, va: u64, npages: u64, do_free: bool) {
    if va % PGSIZE != 0 {
        panic!("uvmunmap: not aligned");
    }

    let mut a = va;
    let end = va + npages * PGSIZE;

    while a < end {
        let pte = walk(pagetable, a, false);
        if pte.is_null() {
            panic!("uvmunmap: walk");
        }
        if (*pte & PTE_V) == 0 {
            panic!("uvmunmap: not mapped");
        }
        if pte_flags(*pte) == PTE_V {
            panic!("uvmunmap: not a leaf");
        }
        if do_free {
            let pa = pte2pa(*pte);
            kfree(pa as *mut u8);
        }
        *pte = 0;
        a += PGSIZE;
    }
}

/// create an empty user page table.
/// returns null if out of memory.
pub unsafe fn uvmcreate() -> PageTable {
    let pagetable = kalloc() as PageTable;
    if pagetable.is_null() {
        return ptr::null_mut();
    }
    ptr::write_bytes(pagetable, 0, PGSIZE.try_into().unwrap());
    pagetable
}

/// Load the user initcode into address 0 of pagetable,
/// for the very first process.
/// sz must be less than a page.
pub unsafe fn uvmfirst(pagetable: PageTable, src: *const u8, sz: usize) {
    if sz >= PGSIZE.try_into().unwrap() {
        panic!("uvminit: more than a page");
    }

    let mem = kalloc();
    if mem.is_null() {
        panic!("uvminit: kalloc");
    }

    ptr::write_bytes(mem, 0, PGSIZE.try_into().unwrap());
    mappages(
        pagetable,
        0,
        PGSIZE as u64,
        mem as u64,
        PTE_W | PTE_R | PTE_X | PTE_U,
    );
    ptr::copy_nonoverlapping(src, mem, sz);
}

/// Allocate PTEs and physical memory to grow process from oldsz to
/// newsz, which need not be page aligned.  Returns new size or 0 on error.
pub unsafe fn uvmalloc(pagetable: PageTable, oldsz: u64, newsz: u64, xperm: i32) -> u64 {
    if newsz < oldsz {
        return oldsz;
    }

    let oldsz = pgroundup(oldsz);
    let mut a = oldsz;

    while a < newsz {
        let mem = kalloc();
        if mem.is_null() {
            uvmdealloc(pagetable, a, oldsz);
            return 0;
        }
        ptr::write_bytes(mem, 0, PGSIZE.try_into().unwrap());

        if mappages(
            pagetable,
            a,
            PGSIZE as u64,
            mem as u64,
            PTE_R | PTE_U | xperm as u64,
        ) != 0
        {
            kfree(mem);
            uvmdealloc(pagetable, a, oldsz);
            return 0;
        }
        a += PGSIZE as u64;
    }
    newsz
}

/// Deallocate user pages to bring the process size from oldsz to
/// newsz.  oldsz and newsz need not be page-aligned, nor does newsz
/// need to be less than oldsz.  oldsz can be larger than the actual
/// process size.  Returns the new process size.
pub unsafe fn uvmdealloc(pagetable: PageTable, oldsz: u64, newsz: u64) -> u64 {
    if newsz >= oldsz {
        return oldsz;
    }

    if pgroundup(newsz) < pgroundup(oldsz) {
        let npages = (pgroundup(oldsz) - pgroundup(newsz)) / PGSIZE as u64;
        uvmunmap(pagetable, pgroundup(newsz), npages, true);
    }

    newsz
}

/// Recursively free page-table pages.
/// All leaf mappings must already have been removed.
unsafe fn freewalk(pagetable: PageTable) {
    // there are 2^9 = 512 PTEs in a page table.
    for i in 0..512 {
        let pte = *pagetable.add(i);
        if (pte & PTE_V as usize) != 0 && (pte & (PTE_R | PTE_W | PTE_X) as usize) == 0 {
            // this PTE points to a lower-level page table.
            let child = pte2pa(pte.try_into().unwrap()) as PageTable;
            freewalk(child);
            *pagetable.add(i) = 0;
        } else if (pte & PTE_V as usize) != 0 {
            panic!("freewalk: leaf");
        }
    }
    kfree(pagetable as *mut u8);
}

/// Free user memory pages,
/// then free page-table pages.
pub unsafe fn uvmfree(pagetable: PageTable, sz: u64) {
    if sz > 0 {
        uvmunmap(pagetable, 0, pgroundup(sz) / PGSIZE as u64, true);
    }
    freewalk(pagetable);
}

/// Given a parent process's page table, copy
/// its memory into a child's page table.
/// Copies both the page table and the
/// physical memory.
/// returns 0 on success, -1 on failure.
/// frees any allocated pages on failure.
pub unsafe fn uvmcopy(old: PageTable, new: PageTable, sz: u64) -> i32 {
    let mut i: u64 = 0;
    while i < sz {
        let pte = walk(old, i, false);
        if pte.is_null() {
            panic!("uvmcopy: pte should exist");
        }
        if (*pte & PTE_V) == 0 {
            panic!("uvmcopy: page not present");
        }

        let pa = pte2pa(*pte);
        let flags = pte_flags(*pte);
        let mem = kalloc();

        if mem.is_null() {
            uvmunmap(new, 0, i / PGSIZE as u64, true);
            return -1;
        }

        ptr::copy_nonoverlapping(pa as *const u8, mem, PGSIZE.try_into().unwrap());

        if mappages(new, i, PGSIZE as u64, mem as u64, flags) != 0 {
            kfree(mem);
            uvmunmap(new, 0, i / PGSIZE as u64, true);
            return -1;
        }

        i += PGSIZE as u64;
    }
    0
}

/// mark a PTE invalid for user access.
/// used by exec for the user stack guard page.
pub unsafe fn uvmclear(pagetable: PageTable, va: u64) {
    let pte = walk(pagetable, va, false);
    if pte.is_null() {
        panic!("uvmclear");
    }
    *pte &= !PTE_U;
}

/// Copy from kernel to user.
/// Copy len bytes from src to virtual address dstva in a given page table.
/// Return 0 on success, -1 on error.
pub unsafe fn copyout(
    pagetable: PageTable,
    mut dstva: u64,
    mut src: *const u8,
    mut len: u64,
) -> i32 {
    while len > 0 {
        let va0 = pgrounddown(dstva);
        if va0 >= MAXVA {
            return -1;
        }

        let pte = walk(pagetable, va0, false);
        if pte.is_null() || (*pte & PTE_V) == 0 || (*pte & PTE_U) == 0 || (*pte & PTE_W) == 0 {
            return -1;
        }

        let pa0 = pte2pa(*pte);
        let mut n = PGSIZE as u64 - (dstva - va0);
        if n > len {
            n = len;
        }

        ptr::copy_nonoverlapping(src, (pa0 + (dstva - va0)) as *mut u8, n as usize);

        len -= n;
        src = src.add(n as usize);
        dstva = va0 + PGSIZE as u64;
    }
    0
}

/// Copy from user to kernel.
/// Copy len bytes to dst from virtual address srcva in a given page table.
/// Return 0 on success, -1 on error.
pub unsafe fn copyin(pagetable: PageTable, mut dst: *mut u8, mut srcva: u64, mut len: u64) -> i32 {
    while len > 0 {
        let va0 = pgrounddown(srcva);
        let pa0 = walkaddr(pagetable, va0);
        if pa0 == 0 {
            return -1;
        }

        let mut n = PGSIZE as u64 - (srcva - va0);
        if n > len {
            n = len;
        }

        ptr::copy_nonoverlapping((pa0 + (srcva - va0)) as *const u8, dst, n as usize);

        len -= n;
        dst = dst.add(n as usize);
        srcva = va0 + PGSIZE as u64;
    }
    0
}

/// Copy a null-terminated string from user to kernel.
/// Copy bytes to dst from virtual address srcva in a given page table,
/// until a '\0', or max.
/// Return 0 on success, -1 on error.
pub unsafe fn copyinstr(
    pagetable: PageTable,
    mut dst: *mut u8,
    mut srcva: u64,
    mut max: u64,
) -> i32 {
    let mut got_null = false;

    while !got_null && max > 0 {
        let va0 = pgrounddown(srcva);
        let pa0 = walkaddr(pagetable, va0);
        if pa0 == 0 {
            return -1;
        }

        let mut n = PGSIZE as u64 - (srcva - va0);
        if n > max {
            n = max;
        }

        let mut p = (pa0 + (srcva - va0)) as *const u8;

        for _ in 0..n {
            if *p == 0 {
                *dst = 0;
                got_null = true;
                break;
            } else {
                *dst = *p;
            }

            n -= 1;
            max -= 1;
            p = p.add(1);
            dst = dst.add(1);
        }

        srcva = va0 + PGSIZE as u64;
    }

    if got_null {
        0
    } else {
        -1
    }
}
