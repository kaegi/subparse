// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod common;
pub mod idx;
pub mod microdvd;
pub mod srt;
pub mod ssa;
pub mod vobsub;

use encoding_rs::Encoding;
use errors::*;
use SubtitleFile;

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
/// Calling the function with the full file path or simply a `get_subtitle_format_by_ending(".srt")`
/// both work. Returns `None` if subtitle format could not be recognized.
///
/// Because the `.sub` file ending is ambiguous (both `MicroDVD` and `VobSub` use that ending) the
/// function will return `None` in that case. Instead, use the content-aware `get_subtitle_format`
/// to handle this case correctly.
pub fn get_subtitle_format_by_ending(ending: &str) -> Option<SubtitleFormat> {
    if ending.ends_with(".srt") {
        Some(SubtitleFormat::SubRip)
    } else if ending.ends_with(".ssa") || ending.ends_with(".ass") {
        Some(SubtitleFormat::SubStationAlpha)
    } else if ending.ends_with(".idx") {
        Some(SubtitleFormat::VobSubIdx)
    } else {
        None
    }
}

/// Returns the subtitle format by the file ending.
///
/// Works exactly like `get_subtitle_format_by_ending`, but instead of `None` a `UnknownFileFormat`
/// will be returned (for simpler error handling).
pub fn get_subtitle_format_by_ending_err(ending: &str) -> Result<SubtitleFormat> {
    get_subtitle_format_by_ending(ending).ok_or_else(|| ErrorKind::UnknownFileFormat.into())
}

/// Returns the subtitle format by the file ending and provided content.
///
/// Calling the function with the full file path or simply a `get_subtitle_format(".sub", content)`
/// both work. Returns `None` if subtitle format could not be recognized.
///
/// It works exactly the same as `get_subtitle_format_by_ending` (see documentation), but also handles the  `.sub` cases
/// correctly by using the provided content of the file as secondary info.
pub fn get_subtitle_format(ending: &str, content: &[u8]) -> Option<SubtitleFormat> {
    if ending.ends_with(".sub") {
        // test for VobSub .sub magic number
        if content.iter().take(4).cloned().eq([0x00, 0x00, 0x01, 0xba].iter().cloned()) {
            Some(SubtitleFormat::VobSubSub)
        } else {
            Some(SubtitleFormat::MicroDVD)
        }
    } else {
        get_subtitle_format_by_ending(ending)
    }
}

/// Returns the subtitle format by the file ending and provided content.
///
/// Works exactly like `get_subtitle_format`, but instead of `None` a `UnknownFileFormat`
/// will be returned (for simpler error handling).
pub fn get_subtitle_format_err(ending: &str, content: &[u8]) -> Result<SubtitleFormat> {
    get_subtitle_format(ending, content).ok_or_else(|| ErrorKind::UnknownFileFormat.into())
}

/// This trick works around the limitation, that trait objects can not require Sized (or Clone).
pub trait ClonableSubtitleFile: SubtitleFile + Send + Sync {
    /// Returns a boxed clone
    fn clone_box(&self) -> Box<ClonableSubtitleFile>;
}
impl<T> ClonableSubtitleFile for T
where
    T: SubtitleFile + Clone + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<ClonableSubtitleFile> {
        Box::new(Clone::clone(self))
    }
}

// We can now implement Clone manually by forwarding to clone_box.
impl Clone for Box<ClonableSubtitleFile> {
    fn clone(&self) -> Box<ClonableSubtitleFile> {
        self.clone_box()
    }
}

/// Parse text subtitles, invoking the right parser given by `format`.
///
/// Returns an `Err(ErrorKind::TextFormatOnly)` if attempted on a binary file format.
///
/// # Mandatory format specific options
///
/// See `parse_bytes`.
pub fn parse_str(format: SubtitleFormat, content: &str, fps: f64) -> Result<Box<ClonableSubtitleFile>> {
    match format {
        SubtitleFormat::SubRip => Ok(Box::new(srt::SrtFile::parse(content)?)),
        SubtitleFormat::SubStationAlpha => Ok(Box::new(ssa::SsaFile::parse(content)?)),
        SubtitleFormat::VobSubIdx => Ok(Box::new(idx::IdxFile::parse(content)?)),
        SubtitleFormat::VobSubSub => Err(ErrorKind::TextFormatOnly.into()),
        SubtitleFormat::MicroDVD => Ok(Box::new(microdvd::MdvdFile::parse(content, fps)?)),
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
pub fn parse_bytes(format: SubtitleFormat, content: &[u8], encoding: &'static Encoding, fps: f64) -> Result<Box<ClonableSubtitleFile>> {
    match format {
        SubtitleFormat::SubRip => Ok(Box::new(srt::SrtFile::parse(&decode_bytes_to_string(content, encoding)?)?)),
        SubtitleFormat::SubStationAlpha => Ok(Box::new(ssa::SsaFile::parse(&decode_bytes_to_string(content, encoding)?)?)),
        SubtitleFormat::VobSubIdx => Ok(Box::new(idx::IdxFile::parse(&decode_bytes_to_string(content, encoding)?)?)),
        SubtitleFormat::VobSubSub => Ok(Box::new(vobsub::VobFile::parse(content)?)),
        SubtitleFormat::MicroDVD => Ok(Box::new(microdvd::MdvdFile::parse(&decode_bytes_to_string(content, encoding)?, fps)?)),
    }
}
