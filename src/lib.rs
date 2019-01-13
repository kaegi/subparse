// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! This crate provides a common interface for popular subtitle formats (`.srt`, `.ssa`, `.ass`, `.idx`, `.sub`).
//!
//! Files can be parsed, modified and saved again - some formats can be created from scratch.
//! The focus is on non-destructive parsing, meaning that formatting and other information are preserved
//! if not explicitely changed.

#[macro_use]
extern crate error_chain;
extern crate combine;
extern crate encoding_rs;
extern crate itertools;
extern crate vobsub;

mod formats;

/// Types that represent a time point, duration and time span.
pub mod timetypes;

/// Error-chain generated error types.
pub mod errors;

use errors::*;
pub use formats::idx::IdxFile;
pub use formats::microdvd::MdvdFile;
pub use formats::srt::SrtFile;
pub use formats::ssa::SsaFile;
pub use formats::vobsub::VobFile;
pub use formats::SubtitleFormat;
pub use formats::{
    get_subtitle_format, get_subtitle_format_by_ending, get_subtitle_format_by_ending_err, get_subtitle_format_err, parse_bytes, parse_str,
};
use timetypes::TimeSpan;

/// This trait represents the generic interface for reading and writing subtitle information across all subtitle formats.
///
/// This trait allows you to read, change and rewrite the subtitle file.
pub trait SubtitleFile {
    /// The subtitle entries can be changed by calling `update_subtitle_entries()`.
    fn get_subtitle_entries(&self) -> Result<Vec<SubtitleEntry>>;

    /// Set the entries from the subtitle entries from the `get_subtitle_entries()`.
    ///
    /// The length of the given input slice should always match the length of the vector length from
    /// `get_subtitle_entries()`. This function can not delete/create new entries, but preserves
    /// everything else in the file (formatting, authors, ...).
    ///
    /// If the input entry has `entry.line == None`, the line will not be overwritten.
    ///
    /// Be aware that .idx files cannot save time_spans_ (a subtitle will be shown between two
    /// consecutive timepoints/there are no separate starts and ends) - so the timepoint will be set
    /// to the start of the corresponding input-timespan.
    fn update_subtitle_entries(&mut self, i: &[SubtitleEntry]) -> Result<()>;

    /// Returns a byte-stream in the respective format (.ssa, .srt, etc.) with the
    /// (probably) altered information.
    fn to_data(&self) -> Result<Vec<u8>>;
}

/// The data which can be read from/written to a subtitle file.
#[derive(Debug)]
pub struct SubtitleEntry {
    /// The duration for which the current subtitle will be shown.
    pub timespan: TimeSpan,

    // TODO: to Vec<String>
    /// The text which will be shown in this subtitle. Be aware that
    /// for example VobSub files (and any other image based format)
    /// will have `None` as value.
    pub line: Option<String>,
}

impl SubtitleEntry {
    /// Create subtitle entry with text.
    fn new(timespan: TimeSpan, line: String) -> SubtitleEntry {
        SubtitleEntry {
            timespan: timespan,
            line: Some(line),
        }
    }
}

impl From<TimeSpan> for SubtitleEntry {
    fn from(f: TimeSpan) -> SubtitleEntry {
        SubtitleEntry { timespan: f, line: None }
    }
}
