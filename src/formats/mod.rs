// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


pub mod srt;
pub mod ssa;
pub mod idx;
pub mod microdvd;
pub mod vobsub;
pub mod common;

use SubtitleFile;
use {ParseSubtitle, ParseSubtitleString};
use errors::*;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// All formats which are supported by this library.
pub enum SubtitleFormat {
    /// .srt file
    SubRip,

    /// .ssa/.ass file
    SubStationAlpha,

    /// .idx file
    VobSubIdx,

    /// .sub file (VobSub/binary)
    VobSubSub,

    /// .sub file (MicroDVD/text)
    MicroDVD,
}

impl SubtitleFormat {
    /// Get a descriptive string for the format like `".srt (SubRip)"`.
    pub fn get_name(&self) -> &'static str {
        match *self {
            SubtitleFormat::SubRip => ".srt (SubRip)",
            SubtitleFormat::SubStationAlpha => ".ssa (SubStation Alpha)",
            SubtitleFormat::VobSubIdx => ".idx (VobSub)",
            SubtitleFormat::VobSubSub => ".sub (VobSub)",
            SubtitleFormat::MicroDVD => ".sub (MicroDVD)",
        }
    }
}


/// Returns the subtitle format by the file ending (`Option` as return type).
///
/// Calling the function with the full file path or simply a `get_subtitle_format_by_ending(".srt")`
/// both work. Returns `None` if subtitle format could not be reckognized.
pub fn get_subtitle_format_by_ending(path: &str) -> Option<SubtitleFormat> {

    if path.ends_with(".srt") {
        Some(SubtitleFormat::SubRip)
    } else if path.ends_with(".ssa") || path.ends_with(".ass") {
        Some(SubtitleFormat::SubStationAlpha)
    } else if path.ends_with(".idx") {
        Some(SubtitleFormat::VobSubIdx)
    } else if path.ends_with(".sub") {
        Some(SubtitleFormat::MicroDVD)
    } else {
        None
    }
}

/// Returns the subtitle format by the file ending (`Result` as return type).
///
/// Calling the function with the full file path or simply a `get_subtitle_format_by_ending(".srt")`
/// both work. Returns `UnknownFileFormat` if subtitle format could not be reckognized.
pub fn get_subtitle_format_by_ending_err(path: &str) -> Result<SubtitleFormat> {
    match get_subtitle_format_by_ending(path) {
        Some(format) => Ok(format),
        None => Err(Error::from(ErrorKind::UnknownFileFormat)),
    }
}

// This trick works around the limitation, that trait objects can not require Sized (or Clone).
pub trait ClonableSubtitleFile: SubtitleFile {
    fn clone(&self) -> Box<ClonableSubtitleFile>;
}
impl<T> ClonableSubtitleFile for T
    where T: SubtitleFile + Clone + 'static
{
    fn clone(&self) -> Box<ClonableSubtitleFile> {
        Box::new(Clone::clone(self))
    }
}

/// Parse text subtitles, invoking the right parser given by `format`.
///
/// Returns an `Err(ErrorKind::TextFormatOnly)` if attempted on a binary file format.
pub fn parse_file_from_string(format: SubtitleFormat, content: String) -> Result<Box<ClonableSubtitleFile>> {
    match format {
        SubtitleFormat::SubRip => Ok(Box::new(srt::SrtFile::parse_from_string(content)?)),
        SubtitleFormat::SubStationAlpha => Ok(Box::new(ssa::SsaFile::parse_from_string(content)?)),
        SubtitleFormat::VobSubIdx => Ok(Box::new(idx::IdxFile::parse_from_string(content)?)),
        SubtitleFormat::VobSubSub => Err(ErrorKind::TextFormatOnly.into()),
        SubtitleFormat::MicroDVD => Ok(Box::new(microdvd::MdvdFile::parse_from_string(content)?)),
    }
}

/// Parse all subtitle formats, invoking the right parser given by `format`.
///
/// Decodes the bytes to a String for text formats (assuming utf-8 encoding).
pub fn parse_file(format: SubtitleFormat, content: &[u8]) -> Result<Box<ClonableSubtitleFile>> {
    match format {
        SubtitleFormat::SubRip => Ok(Box::new(srt::SrtFile::parse(content)?)),
        SubtitleFormat::SubStationAlpha => Ok(Box::new(ssa::SsaFile::parse(content)?)),
        SubtitleFormat::VobSubIdx => Ok(Box::new(idx::IdxFile::parse(content)?)),
        SubtitleFormat::VobSubSub => Ok(Box::new(vobsub::VobFile::parse(content)?)),
        SubtitleFormat::MicroDVD => Ok(Box::new(microdvd::MdvdFile::parse(content)?)),
    }
}
