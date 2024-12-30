use sbi_rt::{self, Shutdown, NoReason, SystemFailure};

pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

pub fn shutdown(failure: bool) -> ! {
    if !failure {
        sbi_rt::system_reset(Shutdown, NoReason);
    } else {
        sbi_rt::system_reset(Shutdown, SystemFailure);
    }
    unreachable!()
}