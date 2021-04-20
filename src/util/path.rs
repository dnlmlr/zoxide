use anyhow::{Context, Result};

use std::env;
use std::path::{Path, PathBuf};

pub fn canonicalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    dunce::canonicalize(&path)
        .with_context(|| format!("could not resolve path: {}", path.as_ref().display()))
}

pub fn current_dir() -> Result<PathBuf> {
    env::current_dir().context("could not get current directory")
}

/// Lexically extracts the filename from a path. Returns an empty string if not
/// found.
pub fn filename_str<S: AsRef<str>>(path: &S) -> &str {
    let mut path = path.as_ref();
    if cfg!(unix) {
        const SEPARATOR: char = '/';
        path = path.trim_end_matches(SEPARATOR);
        // NOTE: can be written using str::rsplit_once once it is stabilized.
        match path.rfind(SEPARATOR) {
            Some(idx) => &path[idx + 1..],
            None => path,
        }
    } else {
        Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
    }
}

/// Determines if zoxide can support the given path. It is assumed that the
/// path is a completely resolved absolute path.
///
/// - On UNIX, all paths are supported.
/// - On Windows, all Win32 drive absolute paths (`C:\foo\bar`) are supported.
///   <https://googleprojectzero.blogspot.com/2016/02/the-definitive-guide-on-win32-to-nt.html>
///   <https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file>
pub fn is_supported<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    if cfg!(windows) {
        use std::path::{Component, Prefix};
        let mut components = path.components();
        match components.next() {
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)) => {
                components.next() == Some(Component::RootDir)
            }
            _ => return false,
        }
    } else {
        true
    }
}

/// Resolves all path components lexically (without accessing the filesystem).
///
/// This is similar to `realpath -ms`, except that it returns the current
/// directory when given an empty path, whereas `realpath` would return
/// `ENOENT: no such file or directory`.
///
/// This function is only available on UNIX.
#[cfg(unix)]
pub fn normalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    const SEPARATOR: u8 = b'/';
    const IS_SEPARATOR: fn(&u8) -> bool = |&b| b == SEPARATOR;

    let path = path.as_ref().as_os_str().as_bytes();
    let mut result;

    // If path is not absolute, push the current directory to result.
    if path.first() == Some(&SEPARATOR) {
        result = vec![SEPARATOR];
    } else {
        let current_dir = current_dir()?;
        result = current_dir.into_os_string().into_vec();
    }

    // Iterate through path components, and push / pop components to result
    // accordingly.
    for component in path.split(IS_SEPARATOR) {
        match component {
            b"" | b"." => (),
            b".." => {
                if let Some(idx) = result.iter().rposition(IS_SEPARATOR) {
                    result.truncate(idx.max(1))
                }
            }
            _ => {
                if result.last() != Some(&SEPARATOR) {
                    result.push(SEPARATOR);
                }
                result.extend_from_slice(component);
            }
        }
    }

    Ok(OsString::from_vec(result).into())
}

pub fn to_str<P: AsRef<Path>>(path: &P) -> Result<&str> {
    let path = path.as_ref();
    path.to_str()
        .with_context(|| format!("invalid unicode in path: {}", path.display()))
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn test_filename_str() {
        const TEST_CASES: &[(&str, &str)] = &[
            ("", ""),
            ("/", ""),
            ("//", ""),
            ("///", ""),
            ("///foo/.//bar//", "bar"),
            ("///foo/.//bar//.//..//.//baz", "baz"),
        ];
        for &(input, expected) in TEST_CASES {
            assert_eq!(super::filename_str(&input), expected)
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_normalize() {
        use std::os::unix::ffi::OsStrExt;

        const TEST_CASES: &[(&str, &str)] = &[
            ("/", "/"),
            ("//", "/"),
            ("///", "/"),
            ("///foo/.//bar//", "/foo/bar"),
            ("///foo/.//bar//.//..//.//baz", "/foo/baz"),
            ("///..//./foo/.//bar", "/foo/bar"),
            ("/foo/../../../bar", "/bar"),
            ("/a/b/c/../../../x/y/z", "/x/y/z"),
            ("///..//./foo/.//bar", "/foo/bar"),
        ];
        for &(input, expected) in TEST_CASES {
            assert_eq!(
                super::normalize(input).unwrap().as_os_str().as_bytes(),
                expected.as_bytes(),
            )
        }

        // Empty path should not return an error.
        assert!(super::normalize("").is_ok());
    }
}
