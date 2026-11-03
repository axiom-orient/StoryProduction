//! Bounded operating-system process identity used for stale-lock recovery.

use std::fs;
use std::io;
use std::sync::OnceLock;

#[derive(Debug)]
pub(super) enum OwnerProbe {
    Live,
    Dead,
    Reused,
    Unknown,
}

pub(super) fn probe_owner(pid: u32, expected_start_identity: &str) -> OwnerProbe {
    if pid == 0 || pid > i32::MAX as u32 {
        return OwnerProbe::Unknown;
    }
    if expected_start_identity.starts_with("unavailable:") {
        return OwnerProbe::Unknown;
    }
    let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if rc != 0 {
        return match io::Error::last_os_error().raw_os_error() {
            Some(libc::ESRCH) => OwnerProbe::Dead,
            _ => OwnerProbe::Unknown,
        };
    }
    match process_start_identity(pid) {
        Some(actual) if actual == expected_start_identity => OwnerProbe::Live,
        Some(_) => OwnerProbe::Reused,
        None => OwnerProbe::Unknown,
    }
}

pub(super) fn current_process_start_identity() -> String {
    static IDENTITY: OnceLock<String> = OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            process_start_identity(std::process::id()).unwrap_or_else(|| {
                // A PID without a verifiable birth identity is never safe to
                // classify as a live or reused owner.  Recording an explicit
                // unavailable marker lets later contenders take the manual
                // recovery path instead of guessing across PID reuse.
                format!("unavailable:{}", std::process::id())
            })
        })
        .clone()
}

fn process_start_identity(pid: u32) -> Option<String> {
    let path = format!("/proc/{pid}/stat");
    if let Ok(contents) = fs::read_to_string(path) {
        let after_name = contents.rsplit_once(") ")?.1;
        let fields: Vec<&str> = after_name.split_whitespace().collect();
        // `starttime` is field 22; after the comm field, it is index 19.
        return fields.get(19).map(|value| format!("proc-start:{value}"));
    }
    #[cfg(target_os = "macos")]
    if let Some(identity) = mac_process_start_identity(pid) {
        return Some(identity);
    }
    // A few POSIX hosts do not expose a process birth time.  Never turn that
    // absence into a PID-only identity: the caller must require manual
    // recovery because PID reuse cannot be disproved.
    None
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[allow(dead_code)]
struct ProcBsdInfoPrefix {
    pbi_flags: u32,
    pbi_status: u32,
    pbi_xstatus: u32,
    pbi_pid: u32,
    pbi_ppid: u32,
    pbi_uid: u32,
    pbi_gid: u32,
    pbi_ruid: u32,
    pbi_rgid: u32,
    pbi_svuid: u32,
    pbi_svgid: u32,
    rfu_1: u32,
    pbi_comm: [u8; 16],
    pbi_name: [u8; 32],
    pbi_nfiles: u32,
    pbi_pgid: u32,
    pbi_pjobc: u32,
    e_tdev: u32,
    e_tpgid: u32,
    pbi_nice: i32,
    pbi_start_tvsec: u64,
    pbi_start_tvusec: u64,
}

#[cfg(target_os = "macos")]
#[link(name = "proc")]
extern "C" {
    fn proc_pidinfo(
        pid: libc::pid_t,
        flavor: i32,
        arg: u64,
        buffer: *mut libc::c_void,
        buffersize: i32,
    ) -> i32;
}

#[cfg(target_os = "macos")]
fn mac_process_start_identity(pid: u32) -> Option<String> {
    let mut info = std::mem::MaybeUninit::<ProcBsdInfoPrefix>::zeroed();
    let size = std::mem::size_of::<ProcBsdInfoPrefix>() as i32;
    let result = unsafe {
        proc_pidinfo(
            pid as libc::pid_t,
            3, // PROC_PIDTBSDINFO
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if result < size {
        return None;
    }
    let info = unsafe { info.assume_init() };
    Some(format!(
        "mac-start:{}:{}",
        info.pbi_start_tvsec, info.pbi_start_tvusec
    ))
}
