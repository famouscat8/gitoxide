use std::{borrow::Cow, ops::Deref};

use bstr::{BString, ByteSlice};
use smallvec::SmallVec;

use crate::{
    file,
    file::{Metadata, Section, SectionMut},
    parse,
    parse::{section, Event},
};

pub(crate) mod body;
pub use body::{Body, BodyIter};
use git_features::threading::OwnShared;

use crate::file::write::{extract_newline, platform_newline};

impl<'a> Deref for Section<'a> {
    type Target = Body<'a>;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

/// Instantiation and conversion
impl<'a> Section<'a> {
    /// Create a new section with the given `name` and optional, `subsection`, `meta`-data and an empty body.
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        subsection: impl Into<Option<Cow<'a, str>>>,
        meta: impl Into<OwnShared<file::Metadata>>,
    ) -> Result<Self, parse::section::header::Error> {
        Ok(Section {
            header: parse::section::Header::new(name, subsection)?,
            body: Default::default(),
            meta: meta.into(),
        })
    }
}

/// Access
impl<'a> Section<'a> {
    /// Return our header.
    pub fn header(&self) -> &section::Header<'a> {
        &self.header
    }

    /// Return our body, containing all keys and values.
    pub fn body(&self) -> &Body<'a> {
        &self.body
    }

    /// Serialize this type into a `BString` for convenience.
    ///
    /// Note that `to_string()` can also be used, but might not be lossless.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    /// Stream ourselves to the given `out`, in order to reproduce this section mostly losslessly
    /// as it was parsed.
    pub fn write_to(&self, mut out: impl std::io::Write) -> std::io::Result<()> {
        self.header.write_to(&mut out)?;

        if self.body.0.is_empty() {
            return Ok(());
        }

        let nl = self
            .body
            .as_ref()
            .iter()
            .find_map(extract_newline)
            .unwrap_or_else(|| platform_newline());

        if !self
            .body
            .as_ref()
            .iter()
            .take_while(|e| !matches!(e, Event::SectionKey(_)))
            .any(|e| e.to_bstr_lossy().contains_str(nl))
        {
            out.write_all(nl)?;
        }

        let mut saw_newline_after_value = true;
        let mut in_key_value_pair = false;
        for (idx, event) in self.body.as_ref().iter().enumerate() {
            match event {
                Event::SectionKey(_) => {
                    if !saw_newline_after_value {
                        out.write_all(nl)?;
                    }
                    saw_newline_after_value = false;
                    in_key_value_pair = true;
                }
                Event::Newline(_) if !in_key_value_pair => {
                    saw_newline_after_value = true;
                }
                Event::Value(_) | Event::ValueDone(_) => {
                    in_key_value_pair = false;
                }
                _ => {}
            }
            event.write_to(&mut out)?;
            if let Event::ValueNotDone(_) = event {
                if self
                    .body
                    .0
                    .get(idx + 1)
                    .filter(|e| matches!(e, Event::Newline(_)))
                    .is_none()
                {
                    out.write_all(nl)?;
                }
            }
        }
        Ok(())
    }

    /// Return additional information about this sections origin.
    pub fn meta(&self) -> &Metadata {
        &self.meta
    }

    /// Returns a mutable version of this section for adjustment of values.
    pub fn to_mut(&mut self, newline: SmallVec<[u8; 2]>) -> SectionMut<'_, 'a> {
        SectionMut::new(self, newline)
    }
}
