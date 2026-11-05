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

/// `renameat(from.dir, from.name, to.dir, to.name)`: moves the entry itself (a link is moved, not followed).
pub fn rename_at(from: &Parent, to: &Parent) -> io::Result<()> {
    let (a, b) = (cname(&from.name)?, cname(&to.name)?);
    // SAFETY: both names are valid NUL-terminated strings and both directory descriptors stay open for the call
    if unsafe { libc::renameat(from.dir.as_raw_fd(), a.as_ptr(), to.dir.as_raw_fd(), b.as_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn open_dir_flags(dir: &File, name: &OsStr, extra: i32) -> io::Result<File> {
    let c = cname(name)?;
    // SAFETY: as in `open_for_write`
    let fd = unsafe { libc::openat(dir.as_raw_fd(), c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | extra) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` was just returned by openat and nothing else owns it
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Open the directory `name` inside `dir` (a link to a directory is followed).
pub fn open_dir_at(dir: &File, name: &OsStr) -> io::Result<File> {
    open_dir_flags(dir, name, 0)
}

/// `mkdirat(dir, name)`; an existing entry is the caller's `AlreadyExists`.
pub fn make_dir_at(dir: &File, name: &OsStr) -> io::Result<()> {
    let c = cname(name)?;
    // SAFETY: as in `open_for_write`
    if unsafe { libc::mkdirat(dir.as_raw_fd(), c.as_ptr(), 0o777) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Names in an open directory (not `.` and `..`).
fn entries(dir: &File) -> io::Result<Vec<OsString>> {
    // SAFETY: `dup` of a valid descriptor; `fdopendir` takes ownership of the duplicate, `closedir` releases it
    let fd = unsafe { libc::dup(dir.as_raw_fd()) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: as above
    let d = unsafe { libc::fdopendir(fd) };
    if d.is_null() {
        let e = io::Error::last_os_error();
        // SAFETY: `fd` is still ours when fdopendir failed
        unsafe { libc::close(fd) };
        return Err(e);
    }
    let mut out = Vec::new();
    loop {
        // SAFETY: `d` is a valid open directory stream; readdir returns null at the end
        let e = unsafe { libc::readdir(d) };
        if e.is_null() {
            break;
        }
        // SAFETY: `d_name` is a NUL-terminated name inside the returned entry
        let name = unsafe { std::ffi::CStr::from_ptr((*e).d_name.as_ptr()) };
        let b = name.to_bytes();
        if b != b"." && b != b".." {
            out.push(OsStr::from_bytes(b).to_os_string());
        }
    }
    // SAFETY: closes the stream and its descriptor
    unsafe { libc::closedir(d) };
    Ok(out)
}

fn is_dir_nofollow(dir: &File, name: &OsStr) -> io::Result<bool> {
    let c = cname(name)?;
    // SAFETY: a zeroed stat is a valid out-parameter
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: as in `open_for_write`; `st` is a valid out-parameter
    if unsafe { libc::fstatat(dir.as_raw_fd(), c.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((st.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFDIR as u32)
}

fn unlink_at(dir: &File, name: &OsStr, remove_directory: bool) -> io::Result<()> {
    let c = cname(name)?;
    let flag = if remove_directory { libc::AT_REMOVEDIR } else { 0 };
    // SAFETY: as in `open_for_write`
    if unsafe { libc::unlinkat(dir.as_raw_fd(), c.as_ptr(), flag) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn remove_contents(dir: &File) -> io::Result<()> {
    for name in entries(dir)? {
        if is_dir_nofollow(dir, &name)? {
            let sub = open_dir_flags(dir, &name, libc::O_NOFOLLOW)?;
            remove_contents(&sub)?;
            unlink_at(dir, &name, true)?;
        } else {
            unlink_at(dir, &name, false)?;
        }
    }
    Ok(())
}

/// `remove_dir_all` of `p.name` inside `p.dir`, never following a link: a link inside the tree is removed, not entered.
pub fn remove_tree(p: &Parent) -> io::Result<()> {
    let dir = open_dir_flags(&p.dir, &p.name, libc::O_NOFOLLOW)?;
    remove_contents(&dir)?;
    unlink(p, true)
}
