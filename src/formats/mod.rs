// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod common;
pub mod idx;
pub mod microdvd;
pub mod srt;
pub mod ssa;
pub mod vobsub;

use crate::SubtitleEntry;
use crate::errors::*;
use crate::SubtitleFileInterface;
use encoding_rs::Encoding;
use std::ffi::OsStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// All formats which are supported by this library.
pub enum SubtitleFormat {
    /// .srt file
    SubRip,

    /// .ssa/.ass file
    SubStationAlpha,

    /// .idx file
    VobSubIdx,

    /// .sub file (`VobSub`/binary)
    VobSubSub,

    /// .sub file (`MicroDVD`/text)
    MicroDVD,
}

#[derive(Clone, Debug)]
/// Unified wrapper around the all individual subtitle file types.
pub enum SubtitleFile {
    /// .srt file
    SubRipFile(srt::SrtFile),

    /// .ssa/.ass file
    SubStationAlpha(ssa::SsaFile),

    /// .idx file
    VobSubIdxFile(idx::IdxFile),

    /// .sub file (`VobSub`/binary)
    VobSubSubFile(vobsub::VobFile),

    /// .sub file (`MicroDVD`/text)
    MicroDVDFile(microdvd::MdvdFile),
}

impl SubtitleFile {

    /// The subtitle entries can be changed by calling `update_subtitle_entries()`.
    pub fn get_subtitle_entries(&self) -> Result<Vec<SubtitleEntry>> {
        match self {
            SubtitleFile::SubRipFile(f) => f.get_subtitle_entries(),
            SubtitleFile::SubStationAlpha(f) => f.get_subtitle_entries(),
            SubtitleFile::VobSubIdxFile(f) => f.get_subtitle_entries(),
            SubtitleFile::VobSubSubFile(f) => f.get_subtitle_entries(),
            SubtitleFile::MicroDVDFile(f) => f.get_subtitle_entries(),
        }
    }

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
    pub fn update_subtitle_entries(&mut self, i: &[SubtitleEntry]) -> Result<()> {
        match self {
            SubtitleFile::SubRipFile(f) => f.update_subtitle_entries(i),
            SubtitleFile::SubStationAlpha(f) => f.update_subtitle_entries(i),
            SubtitleFile::VobSubIdxFile(f) => f.update_subtitle_entries(i),
            SubtitleFile::VobSubSubFile(f) => f.update_subtitle_entries(i),
            SubtitleFile::MicroDVDFile(f) => f.update_subtitle_entries(i),
        }
    }

    /// Returns a byte-stream in the respective format (.ssa, .srt, etc.) with the
    /// (probably) altered information.
    pub fn to_data(&self) -> Result<Vec<u8>> {
        match self {
            SubtitleFile::SubRipFile(f) => f.to_data(),
            SubtitleFile::SubStationAlpha(f) => f.to_data(),
            SubtitleFile::VobSubIdxFile(f) => f.to_data(),
            SubtitleFile::VobSubSubFile(f) => f.to_data(),
            SubtitleFile::MicroDVDFile(f) => f.to_data(),
        }
    }
}

impl From<srt::SrtFile> for SubtitleFile {
    fn from(f: srt::SrtFile) -> SubtitleFile {
        SubtitleFile::SubRipFile(f)
    }
}

impl From<ssa::SsaFile> for SubtitleFile {
    fn from(f: ssa::SsaFile) -> SubtitleFile {
        SubtitleFile::SubStationAlpha(f)
    }
}

impl From<idx::IdxFile> for SubtitleFile {
    fn from(f: idx::IdxFile) -> SubtitleFile {
        SubtitleFile::VobSubIdxFile(f)
    }
}

impl From<vobsub::VobFile> for SubtitleFile {
    fn from(f: vobsub::VobFile) -> SubtitleFile {
        SubtitleFile::VobSubSubFile(f)
    }
}

impl From<microdvd::MdvdFile> for SubtitleFile {
    fn from(f: microdvd::MdvdFile) -> SubtitleFile {
        SubtitleFile::MicroDVDFile(f)
    }
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

/// Returns the subtitle format by the file ending.
///
/// Calling the function with the full file path or simply a `get_subtitle_format_by_ending(Some("srt"))`
/// both work. Returns `None` if subtitle format could not be recognized.
///
/// Because the `.sub` file ending is ambiguous (both `MicroDVD` and `VobSub` use that ending) the
/// function will return `None` in that case. Instead, use the content-aware `get_subtitle_format`
/// to handle this case correctly.
///
/// `Option` is used to simplify handling with `PathBuf::extension()`.
pub fn get_subtitle_format_by_extension<'a>(extension: Option<&OsStr>) -> Option<SubtitleFormat> {
    let _ext_opt: Option<&OsStr> = extension.into();

    if _ext_opt == Some(OsStr::new("srt")) {
        Some(SubtitleFormat::SubRip)
    } else if _ext_opt == Some(OsStr::new("ssa")) || _ext_opt == Some(OsStr::new("ass")) {
        Some(SubtitleFormat::SubStationAlpha)
    } else if _ext_opt == Some(OsStr::new("idx")) {
        Some(SubtitleFormat::VobSubIdx)
    } else {
        None
    }
}

/// Returns true if the filepath/filename/file-extension is valid for the given subtitle format.
///
/// `Option` is used to simplify handling with `PathBuf::extension()`.
pub fn is_valid_extension_for_subtitle_format(extension: Option<&OsStr>, format: SubtitleFormat) -> bool {
    match format {
        SubtitleFormat::SubRip => extension == Some(OsStr::new("srt")),
        SubtitleFormat::SubStationAlpha => extension == Some(OsStr::new("srt")) || extension == Some(OsStr::new("srt")),
        SubtitleFormat::VobSubIdx => extension == Some(OsStr::new("idx")),
        SubtitleFormat::VobSubSub => extension == Some(OsStr::new("sub")),
        SubtitleFormat::MicroDVD => extension == Some(OsStr::new("sub")),
    }
}

/// Returns the subtitle format by the file extension.
///
/// Works exactly like `get_subtitle_format_by_extension`, but instead of `None` a `UnknownFileFormat`
/// will be returned (for simpler error handling).
///
/// `Option` is used to simplify handling with `PathBuf::extension()`.
pub fn get_subtitle_format_by_extension_err(extension: Option<&OsStr>) -> Result<SubtitleFormat> {
    get_subtitle_format_by_extension(extension).ok_or_else(|| ErrorKind::UnknownFileFormat.into())
}

/// Returns the subtitle format by the file ending and provided content.
///
/// Calling the function with the full file path or simply a `get_subtitle_format(".sub", content)`
/// both work. Returns `None` if subtitle format could not be recognized.
///
/// It works exactly the same as `get_subtitle_format_by_ending` (see documentation), but also handles the  `.sub` cases
/// correctly by using the provided content of the file as secondary info.
///
/// `Option` is used to simplify handling with `PathBuf::extension()`.
pub fn get_subtitle_format(extension: Option<&OsStr>, content: &[u8]) -> Option<SubtitleFormat> {
    if extension == Some(OsStr::new("sub")) {
        // test for VobSub .sub magic number
        if content.iter().take(4).cloned().eq([0x00, 0x00, 0x01, 0xba].iter().cloned()) {
            Some(SubtitleFormat::VobSubSub)
        } else {
            Some(SubtitleFormat::MicroDVD)
        }
    } else {
        get_subtitle_format_by_extension(extension)
    }
}

/// Returns the subtitle format by the file ending and provided content.
///
/// Works exactly like `get_subtitle_format`, but instead of `None` a `UnknownFileFormat`
/// will be returned (for simpler error handling).
pub fn get_subtitle_format_err(extension: Option<&OsStr>, content: &[u8]) -> Result<SubtitleFormat> {
    get_subtitle_format(extension, content).ok_or_else(|| ErrorKind::UnknownFileFormat.into())
}


/// Parse text subtitles, invoking the right parser given by `format`.
///
/// Returns an `Err(ErrorKind::TextFormatOnly)` if attempted on a binary file format.
///
/// # Mandatory format specific options
///
/// See `parse_bytes`.
pub fn parse_str(format: SubtitleFormat, content: &str, fps: f64) -> Result<SubtitleFile> {
    match format {
        SubtitleFormat::SubRip => Ok(srt::SrtFile::parse(content)?.into()),
        SubtitleFormat::SubStationAlpha => Ok(ssa::SsaFile::parse(content)?.into()),
        SubtitleFormat::VobSubIdx => Ok(idx::IdxFile::parse(content)?.into()),
        SubtitleFormat::VobSubSub => Err(ErrorKind::TextFormatOnly.into()),
        SubtitleFormat::MicroDVD => Ok(microdvd::MdvdFile::parse(content, fps)?.into()),
    }
}

/// Helper function for text subtitles for byte-to-text decoding.
fn decode_bytes_to_string(content: &[u8], encoding: &'static Encoding) -> Result<String> {
    let (decoded, _, replaced) = encoding.decode(content);
    if replaced {
        Err(Error::from(ErrorKind::DecodingError))
    } else {
        Ok(decoded.into_owned())
    }
}

/// Parse all subtitle formats, invoking the right parser given by `format`.
///
/// # Mandatory format specific options
///
/// Some subtitle formats require additional parameters to work as expected. If you want to parse
/// a specific format that has no additional parameters, you can use the `parse` function of
/// the respective `***File` struct.
///
/// `encoding`: to parse a text-based subtitle format, a character encoding is needed
///
/// `fps`: this parameter is used for `MicroDVD` `.sub` files. These files do not store timestamps in
/// seconds/minutes/... but in frame numbers. So the timing `0 to 30` means "show subtitle for one second"
/// for a 30fps video, and "show subtitle for half second" for 60fps videos. The parameter specifies how
/// frame numbers are converted into timestamps.
pub fn parse_bytes(format: SubtitleFormat, content: &[u8], encoding: &'static Encoding, fps: f64) -> Result<SubtitleFile> {
    match format {
        SubtitleFormat::SubRip => Ok(srt::SrtFile::parse(&decode_bytes_to_string(content, encoding)?)?.into()),
        SubtitleFormat::SubStationAlpha => Ok(ssa::SsaFile::parse(&decode_bytes_to_string(content, encoding)?)?.into()),
        SubtitleFormat::VobSubIdx => Ok(idx::IdxFile::parse(&decode_bytes_to_string(content, encoding)?)?.into()),
        SubtitleFormat::VobSubSub => Ok(vobsub::VobFile::parse(content)?.into()),
        SubtitleFormat::MicroDVD => Ok(microdvd::MdvdFile::parse(&decode_bytes_to_string(content, encoding)?, fps)?.into()),
    }
}
