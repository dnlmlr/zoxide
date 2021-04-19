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

// Extracts the filename from a path. Returns an empty string if not found.
pub fn filename_str<S: AsRef<str>>(path: &S) -> &str {
    let mut path = path.as_ref();
    if cfg!(unix) {
        if path.ends_with('/') {
            path = &path[..path.len() - 1];
        }
        match path.rfind('/') {
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

// Resolves all path components lexically (without accessing the filesystem).
//
// This is similar to `realpath -ms`, except that it returns the current
// directory when given an empty path, whereas realpath would return
// "ENOENT: no such file or directory".
//
// This function is only available on UNIX systems. Since Windows filesystems
// are typically case-insensitive but case-preserving, normalization would not
// work there.
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
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn test_normalize() {
        const TEST_CASES: &[(&str, &str)] = &[
            ("/", "/"),
            ("//", "//"),
            ("///", "/"),
            ("///foo/.//bar//", "/foo/bar"),
            ("///foo/.//bar//.//..//.//baz", "/foo/baz"),
            ("///..//./foo/.//bar", "/foo/bar"),
            ("/foo/../../../bar", "/bar"),
            ("/a/b/c/../../../x/y/z", "/x/y/z"),
            ("///..//./foo/.//bar", "/foo/bar"),
        ];
        for (input, expected) in TEST_CASES {
            assert_eq!(super::normalize(input).unwrap(), Path::new(expected))
        }

        // Empty path should not return an error.
        assert!(super::normalize("").is_ok());
    }
}
