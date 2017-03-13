// This file is part of the Rust library `subparse`.
//
// Copyright (C) 2017 kaegi
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

pub mod srt;
pub mod ssa;
pub mod idx;
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

    /// .sub file (VobSub)
    VobSubSub,
}

impl SubtitleFormat {
    /// Get a descriptive string for the format like `".srt (SubRip)"`.
    pub fn get_name(&self) -> &'static str {
        match *self {
            SubtitleFormat::SubRip => ".srt (SubRip)",
            SubtitleFormat::SubStationAlpha => ".ssa (SubStation Alpha)",
            SubtitleFormat::VobSubIdx => ".idx (VobSub)",
            SubtitleFormat::VobSubSub => ".sub (VobSub)",
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
        Some(SubtitleFormat::VobSubSub)
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
    }
}
