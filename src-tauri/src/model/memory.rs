//! Total system RAM (Plumed fork, written with Claude Code), to recommend a
//! lighter speech model on machines with little memory. `None` when unknown.

#[cfg(unix)]
pub fn total_memory() -> Option<u64> {
    // SAFETY: sysconf only reads system configuration values.
    let (pages, page_size) = unsafe {
        (
            libc::sysconf(libc::_SC_PHYS_PAGES),
            libc::sysconf(libc::_SC_PAGESIZE),
        )
    };
    match (u64::try_from(pages), u64::try_from(page_size)) {
        (Ok(pages), Ok(page_size)) if pages > 0 && page_size > 0 => pages.checked_mul(page_size),
        _ => None,
    }
}

#[cfg(windows)]
pub fn total_memory() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: MEMORYSTATUSEX is plain data, zeroed then sized as the API requires.
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: `status` is a valid, writable MEMORYSTATUSEX with dwLength set.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    (ok != 0 && status.ullTotalPhys > 0).then_some(status.ullTotalPhys)
}

#[cfg(not(any(unix, windows)))]
pub fn total_memory() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_memory_is_known_on_this_system() {
        // Any machine running the tests has more than 256 MiB.
        assert!(total_memory().is_some_and(|total| total > 256 << 20));
    }
}
