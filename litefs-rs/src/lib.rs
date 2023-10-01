use std::{
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::prelude::OpenOptionsExt},
    path::PathBuf,
    time::Duration,
};

use libc::flock;
use tracing::info;

const HALT_BYTE: i64 = 72;

pub fn halt(database_path: &str) -> Result<bool, (i32, Option<i32>)> {
    // let (flock, lock_type) = get_flock_and_type();
    let lockfile = lockfile(database_path);

    let fd = lockfile.as_raw_fd();
    let mut flock = get_flock();

    // F_OFD_SETLKW
    let flock_command = 38;

    let rv = unsafe { libc::fcntl(fd, flock_command.try_into().unwrap(), &mut flock) };
    if rv == 0 {
        Ok(true)
    } else {
        #[cfg(not(target_os = "macos"))]
        let errno_ptr = unsafe { libc::__errno_location() };
        #[cfg(target_os = "macos")]
        let errno_ptr = unsafe { libc::__error() };

        let errno = if errno_ptr.is_null() {
            None
        } else {
            // *should* be safe here as we checked against NULL pointer..
            Some(unsafe { *errno_ptr })
        };
        Err((rv, errno))
    }
}

pub fn unhalt(database_path: &str) -> Result<bool, (i32, Option<i32>)> {
    let mut flock = get_flock();
    flock.l_type = libc::F_UNLCK;

    let lockfile = lockfile(database_path);
    let fd = lockfile.as_raw_fd();

    // F_OFD_SETLKW
    let flock_command = 38;

    let rv = unsafe { libc::fcntl(fd, flock_command.try_into().unwrap(), &mut flock) };
    if rv == 0 {
        Ok(true)
    } else {
        #[cfg(not(target_os = "macos"))]
        let errno_ptr = unsafe { libc::__errno_location() };
        #[cfg(target_os = "macos")]
        let errno_ptr = unsafe { libc::__error() };

        let errno = if errno_ptr.is_null() {
            None
        } else {
            // *should* be safe here as we checked against NULL pointer..
            Some(unsafe { *errno_ptr })
        };
        Err((rv, errno))
    }
}

pub fn lag(database_path: &str) -> std::io::Result<Duration> {
    let database_path = PathBuf::from(database_path);
    let lagfile = database_path.parent().unwrap().join(".lag");
    let lag = std::fs::read_to_string(lagfile)?;
    info!(lag, "Stringy Lag");

    let lag = lag.trim().parse::<u64>().unwrap();
    let lag = Duration::from_millis(lag);

    Ok(lag)
}

fn lockfile(database_path: &str) -> File {
    let lockfile_path = format!("{database_path}-lock");
    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o666)
        .open(lockfile_path)
        .unwrap();
    fd
}

fn get_flock() -> flock {
    flock {
        l_start: HALT_BYTE,
        l_len: 1,
        l_type: libc::F_WRLCK,
        ..default_flock()
    }
}

fn default_flock() -> flock {
    flock {
        l_type: 0,
        l_whence: 0,
        l_start: 0,
        l_len: 0,
        l_pid: 0,
    }
}
