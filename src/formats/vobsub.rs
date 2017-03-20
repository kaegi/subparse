// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


use {SubtitleEntry, SubtitleFile, SubtitleFormat};
use errors::Result as SubtitleParserResult;
use timetypes::{TimePoint, TimeSpan};
use self::errors::*;

use vobsub;

/// `.sub` `VobSub`-parser-specific errors
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
/// Represents a `.sub` (`VobSub`) file.
pub struct VobFile {
    /// Saves the file data.
    data: Vec<u8>,

    /// The (with vobsub) extracted subtitle lines.
    lines: Vec<VobSubSubtitle>,
}

#[derive(Debug, Clone)]
/// Represents a line in a `VobSub` `.sub` file.
struct VobSubSubtitle {
    timespan: TimeSpan,
}

impl VobFile {
    /// Parse contents of a `VobSub` `.sub` file to `VobFile`.
    pub fn parse(b: &[u8]) -> SubtitleParserResult<Self> {
        let lines = vobsub::subtitles(b)
            .map(|sub_res| -> vobsub::Result<VobSubSubtitle> {
                let sub = sub_res?;

                // only extract the timestamps, discard the big image data
                Ok(VobSubSubtitle {
                    timespan: TimeSpan {
                        start: TimePoint::from_msecs((sub.start_time() * 1000.0) as i64),
                        end: TimePoint::from_msecs((sub.end_time() * 1000.0) as i64),
                    },
                })
            })
            .collect::<vobsub::Result<Vec<VobSubSubtitle>>>()
            .map_err(Error::from)?;

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
               .map(|vsub| {
                   SubtitleEntry {
                       timespan: vsub.timespan,
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
