//! `fs::list`: each folder of the root with sound files is a name, with the
//! paths of its files of the extensions of SuperDirt, without regard to
//! case, in the order of the bytes of the paths. The names come in the order
//! of their bytes. Other files, folders inside a name, names without
//! samples, and names and files that start with a dot stay out. A link
//! counts as what it points to. A missing root is an error.
#![cfg(feature = "fs")]

use std::fs;
use std::io;
use std::path::PathBuf;

use scsynth_dirt::fs::list;

/// A folder of its own under the temporary directory, removed at the end of
/// the test.
struct Root(PathBuf);

impl Root {
    /// An empty folder for the test `test`, unique per test and process.
    fn new(test: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("scsynth-dirt-fs-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        Self(path)
    }

    /// Makes the folder `folder` of the root, with its parents, and an empty
    /// file of each name of `files` in it.
    fn files(&self, folder: &str, files: &[&str]) {
        let folder = self.0.join(folder);
        fs::create_dir_all(&folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
        for file in files {
            let path = folder.join(file);
            fs::write(&path, b"").unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }
    }

    /// The listing of the root.
    fn list(&self) -> Vec<(String, Vec<String>)> {
        list(&self.0).unwrap_or_else(|error| panic!("{}: {error}", self.0.display()))
    }

    /// The listing of the names `names`, each with the paths of its files, in
    /// order.
    fn listing(&self, names: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        names
            .iter()
            .map(|&(name, files)| {
                let paths = files
                    .iter()
                    .map(|file| {
                        let path = self.0.join(name).join(file);
                        path.into_os_string().into_string().expect("a UTF-8 path")
                    })
                    .collect();
                (name.to_owned(), paths)
            })
            .collect()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_files_of_a_name_come_in_the_order_of_the_bytes_of_their_paths() {
    let root = Root::new("order");
    root.files("bd", &["a.aiff", "_x.wav", "9.wav", "A.wav", "10.wav"]);
    assert_eq!(
        root.list(),
        root.listing(&[("bd", &["10.wav", "9.wav", "A.wav", "_x.wav", "a.aiff"])])
    );
}

#[test]
fn an_extension_counts_without_regard_to_case_and_other_files_and_folders_stay_out() {
    let root = Root::new("extensions");
    root.files(
        "bd",
        &[
            "a.WAV",
            "b.Aif",
            "c.AIFF",
            "d.aifc",
            "e.mp3",
            "f.wav.txt",
            "g",
            "h.",
            "wav",
        ],
    );
    root.files("bd/i.wav", &["j.wav"]);
    assert_eq!(
        root.list(),
        root.listing(&[("bd", &["a.WAV", "b.Aif", "c.AIFF", "d.aifc"])])
    );
}

#[test]
fn a_folder_without_samples_and_a_file_of_the_root_stay_out() {
    let root = Root::new("empty");
    root.files("empty", &[]);
    root.files("text", &["notes.txt"]);
    root.files("nested/sub", &["a.wav"]);
    root.files("bd", &["a.wav"]);
    root.files("", &["top.wav"]);
    assert_eq!(root.list(), root.listing(&[("bd", &["a.wav"])]));
}

#[test]
fn the_names_come_in_the_order_of_their_bytes() {
    let root = Root::new("names");
    for name in ["sn", "bd2", "808", "bd", "_x", "Kick"] {
        root.files(name, &["a.wav"]);
    }
    let names: Vec<String> = root.list().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["808", "Kick", "_x", "bd", "bd2", "sn"]);
}

#[test]
fn a_name_or_a_file_that_starts_with_a_dot_stays_out() {
    let root = Root::new("dots");
    root.files(".hidden", &["a.wav"]);
    root.files("bd", &[".a.wav", "._b.wav", "c.wav"]);
    assert_eq!(root.list(), root.listing(&[("bd", &["c.wav"])]));
}

#[cfg(unix)]
#[test]
fn a_link_counts_as_what_it_points_to() {
    use std::os::unix::fs::symlink;

    let root = Root::new("links");
    root.files(".store/kit", &["a.wav"]);
    root.files(".store", &["b.wav"]);
    root.files("bd", &["c.wav"]);
    let store = root.0.join(".store");
    for (target, link) in [
        (store.join("kit"), "kit"),
        (store.join("missing"), "gone"),
        (store.join("b.wav"), "bd/b.wav"),
        (store.join("kit"), "bd/kit.wav"),
        (store.join("missing.wav"), "bd/d.wav"),
    ] {
        let link = root.0.join(link);
        symlink(&target, &link).unwrap_or_else(|error| panic!("{}: {error}", link.display()));
    }
    assert_eq!(
        root.list(),
        root.listing(&[("bd", &["b.wav", "c.wav", "d.wav"]), ("kit", &["a.wav"])])
    );
}

#[test]
fn a_missing_root_is_an_error() {
    let root = Root::new("missing");
    let error = list(&root.0.join("missing")).expect_err("a missing root");
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}
