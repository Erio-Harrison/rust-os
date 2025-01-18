use crate::types::*;
use core::ptr;

/// Fill memory with a constant byte
#[no_mangle]
pub unsafe fn memset(dst: *mut u8, c: i32, n: uint) -> *mut u8 {
    let mut cdst = dst;
    let mut i = 0;
    while i < n {
        *cdst.add(i as usize) = c as u8;
        i += 1;
    }
    dst
}

/// Compare memory areas
#[no_mangle]
pub unsafe fn memcmp(v1: *const u8, v2: *const u8, n: uint) -> i32 {
    let mut s1 = v1;
    let mut s2 = v2;
    let mut n = n;

    while n > 0 {
        n -= 1;
        if *s1 != *s2 {
            return (*s1 as i32) - (*s2 as i32);
        }
        s1 = s1.add(1);
        s2 = s2.add(1);
    }
    0
}

/// Copy memory area
#[no_mangle]
pub unsafe fn memmove(dst: *mut u8, src: *const u8, n: uint) -> *mut u8 {
    if n == 0 {
        return dst;
    }

    let s = src;
    let d = dst;

    if s < d && s.add(n as usize) > d {
        // Must copy backwards
        let mut s = s.add(n as usize);
        let mut d = d.add(n as usize);
        let mut n = n;

        while n > 0 {
            n -= 1;
            s = s.sub(1);
            d = d.sub(1);
            *d = *s;
        }
    } else {
        // Can copy forwards
        let mut s = s;
        let mut d = d;
        let mut n = n;

        while n > 0 {
            n -= 1;
            *d = *s;
            d = d.add(1);
            s = s.add(1);
        }
    }
    dst
}

/// Memory copy (wrapper around memmove)
#[no_mangle]
pub unsafe fn memcpy(dst: *mut u8, src: *const u8, n: uint) -> *mut u8 {
    memmove(dst, src, n)
}

/// Compare n characters of two strings
#[no_mangle]
pub unsafe fn strncmp(p: *const u8, q: *const u8, n: uint) -> i32 {
    let mut p = p;
    let mut q = q;
    let mut n = n;

    while n > 0 && *p != 0 && *p == *q {
        n -= 1;
        p = p.add(1);
        q = q.add(1);
    }

    if n == 0 {
        return 0;
    }
    (*p as i32) - (*q as i32)
}

/// Copy n characters from string src to dst
#[no_mangle]
pub unsafe fn strncpy(s: *mut u8, t: *const u8, n: i32) -> *mut u8 {
    let os = s;
    let mut s = s;
    let mut t = t;
    let mut n = n;

    while n > 0 && *t != 0 {
        *s = *t;
        s = s.add(1);
        t = t.add(1);
        n -= 1;
    }
    while n > 0 {
        *s = 0;
        s = s.add(1);
        n -= 1;
    }
    os
}

/// Safe string copy with guaranteed null termination
#[no_mangle]
pub unsafe fn safestrcpy(s: *mut u8, t: *const u8, n: i32) -> *mut u8 {
    let os = s;
    let mut s = s;
    let mut t = t;
    let mut n = n;

    if n <= 0 {
        return os;
    }

    while n > 1 && *t != 0 {
        *s = *t;
        s = s.add(1);
        t = t.add(1);
        n -= 1;
    }
    *s = 0;
    os
}

/// Get string length
#[no_mangle]
pub unsafe fn strlen(s: *const u8) -> i32 {
    let mut n = 0;
    while *s.add(n as usize) != 0 {
        n += 1;
    }
    n
}
