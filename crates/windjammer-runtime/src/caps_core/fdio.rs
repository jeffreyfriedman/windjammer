//! Descriptor-relative file system calls for the checks that must not resolve a name twice (unix only).
//!
//! A write cannot be decided on an open file the way a read can (opening for write truncates or creates), so the decision is made
//! on the **parent directory**: open the directory, take its canonical path from the descriptor, check `parent/name`, then act on
//! `name` relative to that same descriptor (`openat`, `mkdirat`, `unlinkat`) with `O_NOFOLLOW` on the last component. The parent
//! directory the check saw is the directory the operation uses; a link swapped into any component of the parent path afterwards
//! changes nothing, and a link swapped in as the last component makes the call fail with `ELOOP`, which the caller resolves and
//! checks again.

use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// The canonical path of an open file, from the descriptor (macOS `F_GETPATH`, Linux `/proc/self/fd`).
pub fn path_of_open_file(f: &File) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let mut buf = [0u8; 1024];
        // SAFETY: F_GETPATH writes a NUL-terminated path of at most PATH_MAX (1024) bytes into the buffer
        let r = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETPATH, buf.as_mut_ptr()) };
        if r < 0 {
            return None;
        }
        let n = buf.iter().position(|&b| b == 0)?;
        return Some(PathBuf::from(OsStr::from_bytes(&buf[..n])));
    }
    #[cfg(target_os = "linux")]
    {
        return std::fs::read_link(format!("/proc/self/fd/{}", f.as_raw_fd())).ok();
    }
    #[allow(unreachable_code)]
    {
        let _ = f;
        None
    }
}

/// An open directory and the name inside it that an operation will use.
pub struct Parent {
    pub dir: File,
    /// Canonical path of `dir`, from its descriptor.
    pub canon: PathBuf,
    pub name: OsString,
}

/// Open the parent directory of the absolute, lexically normalized path `lex`.
pub fn open_parent(lex: &Path) -> io::Result<Parent> {
    let (Some(parent), Some(name)) = (lex.parent(), lex.file_name()) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a path with no parent directory or no final name"));
    };
    let dir = File::open(parent)?;
    if !dir.metadata()?.is_dir() {
        return Err(io::Error::from_raw_os_error(libc::ENOTDIR));
    }
    let canon = path_of_open_file(&dir).ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "cannot name an open directory on this platform"))?;
    Ok(Parent { dir, canon, name: name.to_os_string() })
}

fn cname(name: &OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a name with a NUL byte"))
}

/// `openat(dir, name, O_WRONLY | O_CREAT | O_NOFOLLOW | ...)`: create or truncate (`append == false`), or append.
pub fn open_for_write(p: &Parent, append: bool) -> io::Result<File> {
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | if append { libc::O_APPEND } else { libc::O_TRUNC };
    let c = cname(&p.name)?;
    // SAFETY: `c` is a valid NUL-terminated string and the directory descriptor stays open for the call
    let fd = unsafe { libc::openat(p.dir.as_raw_fd(), c.as_ptr(), flags, 0o666 as libc::c_uint) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` was just returned by openat and nothing else owns it
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub fn make_dir(p: &Parent) -> io::Result<()> {
    let c = cname(&p.name)?;
    // SAFETY: as in `open_for_write`
    if unsafe { libc::mkdirat(p.dir.as_raw_fd(), c.as_ptr(), 0o777) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `unlinkat(dir, name, 0)` removes a file or a link (never follows a link); `remove_directory` removes an empty directory.
pub fn unlink(p: &Parent, remove_directory: bool) -> io::Result<()> {
    let c = cname(&p.name)?;
    let flag = if remove_directory { libc::AT_REMOVEDIR } else { 0 };
    // SAFETY: as in `open_for_write`
    if unsafe { libc::unlinkat(p.dir.as_raw_fd(), c.as_ptr(), flag) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn is_symlink_loop(e: &io::Error) -> bool {
    e.raw_os_error() == Some(libc::ELOOP)
}
