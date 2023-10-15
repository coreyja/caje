use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::prelude::OpenOptionsExt},
    path::PathBuf,
    time::Duration,
};

use libc::flock;
use thiserror::Error;
use tracing::info;

const HALT_BYTE: i64 = 72;

pub fn halt(lockfile: &File) -> Result<(), FlockError> {
    // let (flock, lock_type) = get_flock_and_type();

    let fd = lockfile.as_raw_fd();
    let mut flock = get_flock();

    // F_OFD_SETLKW = 38
    let rv = unsafe { libc::fcntl(fd, 38, &mut flock) };
    if rv == 0 {
        Ok(())
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
        Err(FlockError { errno, rv })
    }
}

#[derive(Error, Debug)]
pub struct FlockError {
    pub errno: Option<i32>,
    pub rv: i32,
}

impl Display for FlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Flock Error | rv={} errno={}",
            self.rv,
            self.errno.map(|x| x.to_string()).unwrap_or_default()
        )
    }
}

pub fn unhalt(lockfile: &File) -> Result<(), FlockError> {
    let mut flock = get_flock();
    flock.l_type = libc::F_UNLCK.try_into().unwrap();

    let fd = lockfile.as_raw_fd();

    let rv = unsafe { libc::fcntl(fd, 38, &mut flock) };
    if rv == 0 {
        Ok(())
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
        Err(FlockError { errno, rv })
    }
}

pub fn lag(database_path: &str) -> std::io::Result<Duration> {
    let database_path = PathBuf::from(database_path);
    let parent = database_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Couldnt get parent directory of DB",
        )
    })?;

    let lagfile = parent.join(".lag");
    let lag = std::fs::read_to_string(lagfile)?;
    info!(lag, "Stringy Lag");

    let lag = lag.trim().parse::<u64>().unwrap();
    let lag = Duration::from_millis(lag);

    Ok(lag)
}

pub fn lockfile(database_path: &str) -> Result<std::fs::File, std::io::Error> {
    let lockfile_path = format!("{database_path}-lock");
    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o666)
        .open(lockfile_path);
    fd
}

fn get_flock() -> flock {
    flock {
        l_start: HALT_BYTE,
        l_len: 1,
        l_type: libc::F_WRLCK.try_into().unwrap(),
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
