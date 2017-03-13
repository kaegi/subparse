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

use {ParseSubtitle, SubtitleEntry, SubtitleFile, SubtitleFormat};
use errors::Result as SubtitleParserResult;
use timetypes::{TimePoint, TimeSpan};
use self::errors::*;

use vobsub;
use vobsub::Subtitle as VobSubSubtitle;

/// `.sub` VobSub-parser-specific errors
#[allow(missing_docs)]
pub mod errors {
    use vobsub;

    // see https://docs.rs/error-chain
    error_chain! {
        foreign_links {
            VobSubError(vobsub::Error)
            /// Error from the `vobsub` crate
            ;
        }
    }
}


#[derive(Debug, Clone)]
/// Represents a `.sub` (VobSub) file.
pub struct VobFile {
    /// Saves the file data.
    data: Vec<u8>,

    /// The (with vobsub) extracted subtitle lines.
    lines: Vec<VobSubSubtitle>,
}

impl ParseSubtitle for VobFile {
    fn parse(b: &[u8]) -> SubtitleParserResult<Self> {
        let lines = vobsub::subtitles(b)
            .collect::<vobsub::Result<Vec<VobSubSubtitle>>>()
            .map_err(|e| Error::from(e))?;

        Ok(VobFile {
            data: b.to_vec(),
            lines: lines,
        })
    }
}

impl SubtitleFile for VobFile {
    fn get_subtitle_entries(&self) -> SubtitleParserResult<Vec<SubtitleEntry>> {
        Ok(self.lines
               .iter()
               .map(|s| {
                   TimeSpan {
                       start: TimePoint::from_msecs((s.start_time * 1000.0) as i64),
                       end: TimePoint::from_msecs((s.end_time * 1000.0) as i64),
                   }
               })
               .map(|ts| {
                   SubtitleEntry {
                       timespan: ts,
                       line: None,
                   }
               })
               .collect())
    }

    fn update_subtitle_entries(&mut self, _: &[SubtitleEntry]) -> SubtitleParserResult<()> {
        Err(::errors::ErrorKind::UpdatingEntriesNotSupported(SubtitleFormat::VobSubSub).into())
    }

    fn to_data(&self) -> SubtitleParserResult<Vec<u8>> {
        Ok(self.data.clone())
    }
}
