//! A folder of samples, as SuperDirt lists it: [`list`].

use std::ffi::OsString;
use std::fs::DirEntry;
use std::io;
use std::path::Path;

/// The extensions of the sound files that SuperDirt reads: `fileExtensions`
/// of `DirtSoundLibrary`.
const EXTENSIONS: [&str; 4] = ["wav", "aif", "aiff", "aifc"];

/// The names and the samples of the folder `root`, as SuperDirt lists
/// `root/*` in `DirtSoundLibrary.loadSoundFiles`, with `pathMatch`, which is
/// glob. Each item is a name and the paths of its samples, as
/// [`Library::add`](crate::Library::add) takes them.
///
/// - Each folder inside `root` is a name. Its samples are its files with the
///   extension `wav`, `aif`, `aiff` or `aifc`, without regard to case. A
///   folder inside a name stays out, and so does a name without samples.
/// - A name or a file that starts with a dot stays out, as `*` of glob does
///   not match it.
/// - A link counts as what it points to, and a link that points nowhere
///   counts as a file.
/// - The names come in the order of their bytes, and the samples of a name in
///   the order of the bytes of their paths.
/// - A path is `root` joined with the name and the file. The function does
///   not make it absolute.
///
/// # Errors
///
/// The error of the file system for a folder that does not read, such as
/// [`io::ErrorKind::NotFound`] for a missing `root`.
/// [`io::ErrorKind::InvalidData`] for a name or a path that is not UTF-8.
pub fn list(root: &Path) -> io::Result<Vec<(String, Vec<String>)>> {
    let mut names = Vec::new();
    for entry in entries(root)? {
        let folder = entry.path();
        if is_folder(&folder) {
            let paths = samples(&folder)?;
            if !paths.is_empty() {
                names.push((utf8(entry.file_name())?, paths));
            }
        }
    }
    names.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    Ok(names)
}

/// The paths of the samples in the folder of a name, in the order of their
/// bytes.
fn samples(folder: &Path) -> io::Result<Vec<String>> {
    let mut paths = Vec::new();
    for entry in entries(folder)? {
        let path = entry.path();
        if is_sound_file(&path) && !is_folder(&path) {
            paths.push(utf8(path.into_os_string())?);
        }
    }
    paths.sort_unstable();
    Ok(paths)
}

/// The entries of `folder` that `*` of glob matches: those whose names do not
/// start with a dot.
fn entries(folder: &Path) -> io::Result<Vec<DirEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        if !entry.file_name().as_encoded_bytes().starts_with(b".") {
            entries.push(entry);
        }
    }
    Ok(entries)
}

/// Whether `path` is a folder, or a link to a folder.
fn is_folder(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Whether the extension of `path` is one of [`EXTENSIONS`], without regard
/// to case.
fn is_sound_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        EXTENSIONS
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// `text` as a `String`, or an error of the kind `InvalidData` if it is not
/// UTF-8.
fn utf8(text: OsString) -> io::Result<String> {
    text.into_string().map_err(|text| {
        let message = format!("{} is not UTF-8", Path::new(&text).display());
        io::Error::new(io::ErrorKind::InvalidData, message)
    })
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsStr;
    use std::io;
    use std::os::unix::ffi::OsStrExt;

    use super::utf8;

    #[test]
    fn a_name_that_is_not_utf_8_is_invalid_data() {
        let error = utf8(OsStr::from_bytes(b"b\xffd").to_owned()).expect_err("b\\xffd");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "b\u{fffd}d is not UTF-8");
        assert_eq!(utf8("bd".into()).ok().as_deref(), Some("bd"));
    }
}
