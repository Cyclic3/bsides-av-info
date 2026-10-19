use std::{collections::BTreeMap, fmt::{Debug, Display}, fs::File, path::{self, Path, PathBuf}};

use color_eyre::Result;

use crate::event::{EventIndex, IndexedSession};

// pub mod ical;
// pub mod ontime;
pub mod show_file;

/// Enables a slightly less slow version of opening and appending each time
///
/// XXX: assumes that all files in in the same directory, and will panic if `/` is found in the string
/// (for protection against path traversal)
pub struct FileTracker {
    output_dir: PathBuf,
    file_cache: BTreeMap<String, File>
}
impl FileTracker {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            file_cache: Default::default()
        }
    }
    pub fn open(&mut self, name: String) -> Result<&mut File> {
        assert!(!name.contains('/'), "File traversal attack detected in generated file name {name:?}");
        match self.file_cache.entry(name) {
            std::collections::btree_map::Entry::Occupied(occupied_entry) => Ok(occupied_entry.into_mut()),
            std::collections::btree_map::Entry::Vacant(vacant_entry) => {
                let file = std::fs::File::create(self.output_dir.join(vacant_entry.key()))?;
                Ok(vacant_entry.insert(file))
            },
        }
    }
}

// /// A helper struct to allow you l
// pub struct FileNamer<F: Fn(IndexedSession) -> std::fmt::Arguments> {
//     namer: F,
//     output_dir: std::path::PathBuf,
// }
// impl<F: Fn(IndexedSession) -> std::fmt::Arguments> FileNamer<F> {
//     pub fn new(namer: F, output_dir: std::path::PathBuf) -> Self {
//         Self {
//             namer,
//             output_dir,
//             files: Default::default()
//         }
//     }
//     // Name files as day{idx+1}-{track name}{suffix}, eliding components if there is only one
//     pub fn day_track<S: Display>(event: &EventIndex, suffix: S, output_dir: std::path::PathBuf) -> FileNamer<_> {
//         let track_component =
//             if event.tracks().len() == 1 {
//                 |session| format_args!("")
//             }
//             else {
//                 |session: IndexedSession| format_args!("{}", session.track)
//             };

//         FileNamer::new(|session| format_args!("day{}-{}{suffix}", session.day() + 1, session.track_name), output_dir)
//     }

//     pub fn open(&mut self, session: IndexedSession) -> Result<&mut File> {
//         let key = self.namer(session);
//         match self.files.entry(key) {
//             std::collections::btree_map::Entry::Occupied(occupied_entry) => Ok(occupied_entry.into_mut()),
//             std::collections::btree_map::Entry::Vacant(vacant_entry) => {
//                 let file = std::fs::File::create(self.output_dir.join(key.to_owned()))?;
//                 Ok(vacant_entry.insert(file))
//             },
//         }
//     }
// }

pub trait EventFormatter: Debug {
    fn format(&self, event: EventIndex, output_dir: &std::path::Path) -> Result<()>;
}

pub const DEFAULT_FORMATTERS: &[&dyn EventFormatter] = &[
    &show_file::ShowFile,
];

#[cfg(test)]
mod test {
    use color_eyre::Result;
    use itertools::Itertools;
    use std::{collections::{BTreeSet, BinaryHeap}, path::{Path, PathBuf}};
    use crate::{event::test::load_bristol, outputs::show_file::ShowFile};

    use super::*;

    /// Get files in a directory and return them in lexicographical order
    ///
    /// Because of https://github.com/rust-lang/rust/issues/59278, we can't use the more sane choice of a binary heap
    fn get_files(dir: &Path) -> Result<BinaryHeap<PathBuf>> {
        std::fs::read_dir(dir)?
        .map(|dirent| {
            let dirent = dirent?;
            if !dirent.file_type()?.is_file() {
                Ok(None)
            }
            else {
                Ok(Some(dirent.path()))
            }
        })
        .process_results(|i| i.flatten().collect())
    }
    /// Check the contents are the same
    //
    // We could do this a lot more efficiently (two passes with ordered iterators),
    // but this is easier to check that I haven't made a mistake :p
    fn assert_same_files(output_dir: &Path, compare_dir: &Path) {
        // Get a big list of each collection of files, because that makes formatting wayyyy easier
        let generated_files: BTreeMap<PathBuf, PathBuf> =
            get_files(output_dir).expect("Failed to enumerate generated files").into_sorted_vec().into_iter()
            .map(|i|  (i.strip_prefix(output_dir).unwrap().to_owned(), i))
            .collect();
        let expected_files: BTreeMap<PathBuf, PathBuf> =
            get_files(compare_dir).expect("Failed to enumerate expected files").into_sorted_vec().into_iter()
            .map(|i|  (i.strip_prefix(compare_dir).unwrap().to_owned(), i))
            .collect();

        println!("Generated:\n{}", generated_files.keys().format_with("\n", |i, f| f(&format_args!("    {:#?}", i))));
        println!("Expected:\n{}", expected_files.keys().format_with("\n", |i, f| f(&format_args!("    {:#?}", i))));

        // Yes, we could be smarter than this, but that's a lot of effort to save a ~O(n) pass over a small number of small strings
        let files: Vec<&PathBuf> = generated_files.keys().merge(expected_files.keys()).dedup().collect();

        for file in files {
            let generated_path = generated_files.get(file);
            let expected_path = expected_files.get(file);
            // Check the file contents are the same (both rel paths are the same here)
            let generated_file = generated_path.map(std::fs::read).transpose().expect("Could not read generated file");
            let expected_file = expected_path.map(std::fs::read).transpose().expect("Could not read comparison file");
            if generated_file.is_none() {
                println!("{file:?} was not generated");
            }
            else if expected_file.is_none() {
                println!("{file:?} was not supposed to have been generated");
            }
            // For nicer formatting, see if we can make them strings
            if
                let Ok(generated_file) = generated_file.as_ref().map(|i| std::str::from_utf8(i)).unwrap_or(Ok("")) &&
                let Ok(expected_file) = expected_file.as_ref().map(|i| std::str::from_utf8(i)).unwrap_or(Ok(""))
            {
                assert_eq!(generated_file, expected_file, "The generated {file:?} was different that the expected file");
            }
            else {
                assert_eq!(generated_file, expected_file, "The generated {file:?} was different that the expected file");
            }
            // Make sure that we don't compare empty files as equal to empty strings
            if generated_file.is_none() != expected_file.is_none()  {
                // We printed an error message earlier
                panic!()
            }
            println!("Generated {file:?} matched.");
        }
    }
    /// Test a formatter against a known output
    fn test_formatter(format: &impl EventFormatter, input: EventIndex, compare_dir: &std::path::Path) {
        // I'm making the executive decision that we don't care if somehow something creates empty dirs,
        // and only compare files + paths.
        //
        // If you've screwed up so hard that this is an issue for you, consider fixing your code with GNU shred

        // Create a scoped handle for somewhere to dump our generated data
        let output_dir = tempfile::tempdir().expect("Could not create temporary directory for testing formatter");
        format.format(input, output_dir.path()).expect("Failed to format input");
        assert_same_files(output_dir.path(), compare_dir);
    }
    macro_rules! test_data_path {
        ($name: literal) => {
            {
                let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                dir.push(std::concat!("test-data/", $name));
                dir
            }
        };
    }
    #[test]
    fn test_assert_same_files() {
        // The source directory should be equal to itself
        let mut files = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // We're in a workspace, so we have to go up once (https://github.com/rust-lang/cargo/issues/3946)
        files.pop();
        files.push(file!());
        files.pop();
        println!("Comparing source directory {files:?} with iteself");
        // If this somehow damages source files imma be so angry
        assert_same_files(&files, &files);
    }
    #[test]
    fn test_show_file() {
        test_formatter(&ShowFile, load_bristol().finish(), &test_data_path!("show-file"));
    }
}
