use std::fmt::Debug;

use color_eyre::Result;

use crate::event::EventIndex;

// pub mod ical;
// pub mod ontime;
pub mod show_file;

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
    use std::{collections::BTreeSet, path::{Path, PathBuf}};
    use crate::{event::test::load_belfast, outputs::show_file::ShowFile};

    use super::*;

    /// Get files in a directory and return them in lexicographical order
    ///
    /// Because of https://github.com/rust-lang/rust/issues/59278, we can't use the more sane choice of a binary heap
    fn get_files(dir: &Path) -> Result<BTreeSet<PathBuf>> {
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
    // We could do this with streams, but this is easier to check that I haven't made a mistake :p
    fn assert_same_files(output_dir: &Path, compare_dir: &Path) {
        let mut generated_files = get_files(output_dir).expect("Failed to enumerate generated files").into_iter();
        let mut expected_files = get_files(compare_dir).expect("Failed to enumerate expected files").into_iter();
        // Because they are in order containers, they have the same ordering (relative lexicographic ordering is preserved over prefixing)
        loop {
            let (generated_path, expected_path) = match (generated_files.next(), expected_files.next()) {
                (None, None) => break,

                // (Some(generated_path), Some(expected_path))
                (Some(generated_path), None) => panic!("{:?} was unexpected in generated output", generated_path.strip_prefix(output_dir)),

                // ((Some(generated_path), Some(expected_path)) if expected_path > generated_path) |
                (None, Some(expected_path)) => panic!("{:?} was unexpected in generated output", expected_path.strip_prefix(compare_dir)),

                (Some(generated_path), Some(expected_path)) => (generated_path, expected_path)
            };

            // We can work out the position it should have been in the other one to get the correct message
            //
            // It's a max heap, so the one that's more got skipped
            let generated_rel_path = generated_path.strip_prefix(output_dir).unwrap();
            let expected_rel_path = expected_path.strip_prefix(compare_dir).unwrap();
            match generated_rel_path.cmp(expected_rel_path) {
                std::cmp::Ordering::Less => panic!("{expected_rel_path:?} was missing in generated output"),
                std::cmp::Ordering::Greater => panic!("{generated_rel_path:?} was unexpected in generated output"),
                std::cmp::Ordering::Equal => (),
            }


            // Check the file contents are the same (both rel paths are the same here)
            let generated_file = std::fs::read(&generated_path).expect("Could not read generated file");
            let expected_file = std::fs::read(&expected_path).expect("Could not read comparison file");
            // For nicer formatting, see if we can make them strings
            if let Ok(generated_file) = std::str::from_utf8(&generated_file) && let Ok(expected_file) = std::str::from_utf8(&expected_file) {
                assert_eq!(generated_file, expected_file, "The generated {expected_rel_path:?} was different that the expected file");
            }
            else {
                assert_eq!(generated_file, expected_file, "The generated {expected_rel_path:?} was different that the expected file");
            }
            println!("Generated {expected_rel_path:?} matched.");
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
        test_formatter(&ShowFile, load_belfast().finish(), &test_data_path!("show-file"));
    }
}
