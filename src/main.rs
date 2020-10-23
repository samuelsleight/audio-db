use audio_db_tags::{Error as ParseError, Flac, Id3v2};

use std::path::PathBuf;

use ignore::{
    DirEntry, Error as DirError, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder, WalkState,
};

use std::{
    ffi::OsStr,
    io::{stdout, Write},
};

fn main() {
    struct FileParserBuilder {
        next_id: usize,
    }

    enum File {
        Mp3(Id3v2),
        Flac(Flac),
    }

    struct FileParser {
        id: usize,
        results: Vec<(PathBuf, Result<File, ParseError>)>,
    }

    impl<'s> ParallelVisitorBuilder<'s> for FileParserBuilder {
        fn build(&mut self) -> Box<(dyn ignore::ParallelVisitor + 's)> {
            let id = self.next_id;
            self.next_id += 1;

            println!("Creating thread {}", id);

            Box::new(FileParser {
                id,
                results: Vec::new(),
            })
        }
    }

    impl ParallelVisitor for FileParser {
        fn visit(&mut self, entry: Result<DirEntry, DirError>) -> WalkState {
            if let Ok(entry) = entry {
                let path = entry.into_path();

                let result = {
                    let extension = path.extension();

                    if extension == Some(OsStr::new("flac")) {
                        Flac::from_path(&path).map(File::Flac)
                    } else if extension == Some(OsStr::new("mp3")) {
                        Id3v2::from_path(&path).map(File::Mp3)
                    } else {
                        return WalkState::Continue;
                    }
                };

                self.results.push((path, result))
            }

            if self.results.len() % 50 == 0 {
                println!("Thread {}, read {} so far", self.id, self.results.len());
            }

            WalkState::Continue
        }
    }

    impl Drop for FileParser {
        fn drop(&mut self) {
            let out = stdout();
            let mut lock = out.lock();

            for (path, result) in &self.results {
                if let Err(result) = result {
                    writeln!(lock, "Error reading {}: {}", path.display(), result).unwrap();
                }
            }
        }
    }

    WalkBuilder::new(r"C:\Users\Samue\Music\HQ")
        .add(r"C:\Users\Samue\Music\Sorted")
        .threads(8)
        .build_parallel()
        .visit(&mut FileParserBuilder { next_id: 0 });
}
