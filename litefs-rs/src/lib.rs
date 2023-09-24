use std::fs::{File, OpenOptions};

use fcntl::{flock, FcntlError, FlockOperations};

const HALT_BYTE: i64 = 72;

pub fn halt(database_path: &str) -> Result<bool, FcntlError> {
    let (flock, lock_type) = get_flock_and_type();
    let lockfile = lockfile(database_path);

    fcntl::lock_file(&lockfile, Some(flock), Some(lock_type))
}

pub fn unhalt(database_path: &str) -> Result<bool, FcntlError> {
    let flock = get_flock();
    let lockfile = lockfile(database_path);

    fcntl::unlock_file(&lockfile, Some(flock))
}

fn lockfile(database_path: &str) -> File {
    let lockfile_path = format!("{database_path}-lock");
    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lockfile_path)
        .unwrap();
    fd
}

fn get_flock() -> flock {
    get_flock_and_type().0
}

fn get_flock_and_type() -> (flock, fcntl::FcntlLockType) {
    let lock_type = fcntl::FcntlLockType::Write;

    (
        flock {
            l_start: HALT_BYTE,
            l_len: 1,
            l_type: lock_type.into(),
            ..libc::flock::default()
        },
        lock_type,
    )
}
