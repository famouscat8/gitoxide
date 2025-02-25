use std::io;

use bstr::{BString, ByteSlice};
use quick_error::quick_error;

use crate::{
    encode::SPACE,
    tree::{Entry, EntryRef},
    Kind, Tree, TreeRef,
};

quick_error! {
    /// The Error used in [`Tree::write_to()`].
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        NewlineInFilename(name: BString) {
            display("Newlines are invalid in file paths: {:?}", name)
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

/// Serialization
impl crate::WriteTo for Tree {
    /// Serialize this tree to `out` in the git internal format.
    fn write_to(&self, mut out: impl io::Write) -> io::Result<()> {
        debug_assert_eq!(
            &{
                let mut entries_sorted = self.entries.clone();
                entries_sorted.sort();
                entries_sorted
            },
            &self.entries,
            "entries for serialization must be sorted by filename"
        );
        for Entry { mode, filename, oid } in &self.entries {
            out.write_all(mode.as_bytes())?;
            out.write_all(SPACE)?;

            if filename.find_byte(b'\n').is_some() {
                return Err(Error::NewlineInFilename((*filename).to_owned()).into());
            }
            out.write_all(filename)?;
            out.write_all(&[b'\0'])?;

            out.write_all(oid.as_bytes())?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.entries
            .iter()
            .map(|Entry { mode, filename, oid }| mode.as_bytes().len() + 1 + filename.len() + 1 + oid.as_bytes().len())
            .sum()
    }

    fn kind(&self) -> Kind {
        Kind::Tree
    }
}

/// Serialization
impl<'a> crate::WriteTo for TreeRef<'a> {
    /// Serialize this tree to `out` in the git internal format.
    fn write_to(&self, mut out: impl io::Write) -> io::Result<()> {
        debug_assert_eq!(
            &{
                let mut entries_sorted = self.entries.clone();
                entries_sorted.sort();
                entries_sorted
            },
            &self.entries,
            "entries for serialization must be sorted by filename"
        );
        for EntryRef { mode, filename, oid } in &self.entries {
            out.write_all(mode.as_bytes())?;
            out.write_all(SPACE)?;

            if filename.find_byte(b'\n').is_some() {
                return Err(Error::NewlineInFilename((*filename).to_owned()).into());
            }
            out.write_all(filename)?;
            out.write_all(&[b'\0'])?;

            out.write_all(oid.as_bytes())?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.entries
            .iter()
            .map(|EntryRef { mode, filename, oid }| {
                mode.as_bytes().len() + 1 + filename.len() + 1 + oid.as_bytes().len()
            })
            .sum()
    }

    fn kind(&self) -> Kind {
        Kind::Tree
    }
}
