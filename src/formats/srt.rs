// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


use {SubtitleEntry, SubtitleFile};
use errors::Result as SubtitleParserResult;
use formats::common::*;
use timetypes::{TimePoint, TimeSpan};
use self::errors::ErrorKind::*;
use self::errors::*;

use std::iter::once;

use itertools::Itertools;

use combine::char::{char, string};
use combine::combinator::{eof, parser as p, skip_many};
use combine::primitives::Parser;

/// `.srt`-parser-specific errors
#[allow(missing_docs)]
pub mod errors {
    // see https://docs.rs/error-chain/0.8.1/error_chain/
    // this error type might be overkill, but that way it stays consistent with
    // the other parsers
    error_chain! {
        errors {
            ExpectedIndexLine(line: String) {
                display("expected SubRip index line, found '{}'", line)
            }
            ExpectedTimestampLine(line: String) {
                display("expected SubRip timespan line, found '{}'", line)
            }
            ErrorAtLine(line_num: usize) {
                display("parse error at line `{}`", line_num)
            }
        }
    }
}

/// The parsing works as a finite state machine. These are the states in it.
enum SrtParserState {
    // emptyline or index follows
    Emptyline,

    /// timing line follows
    Index(i64),

    /// dialog or emptyline follows
    Timing(i64, TimeSpan),

    /// emptyline follows
    Dialog(i64, TimeSpan, Vec<String>),
}

#[derive(Debug, Clone)]
/// Represents a `.srt` file.
pub struct SrtFile {
    v: Vec<SrtLine>,
}

#[derive(Debug, Clone)]
/// A complete description of one `SubRip` subtitle line.
struct SrtLine {
    /// start and end time of subtitle
    timespan: TimeSpan,

    /// index/number of line
    index: i64,

    /// the dialog/text lines of the `SrtLine`
    texts: Vec<String>,
}

impl SrtFile {
    /// Parse a `.srt` subtitle string to `SrtFile`.
    pub fn parse(s: &str) -> SubtitleParserResult<SrtFile> {
        let file_opt = Self::parse_file(s);
        match file_opt {
            Ok(file) => Ok(file),
            Err(err) => Err(err.into()),
        }
    }
}

/// Implements parse functions.
impl SrtFile {
    fn parse_file(i: &str) -> Result<SrtFile> {
        use self::SrtParserState::*;

        let mut result: Vec<SrtLine> = Vec::new();

        // remove utf-8 bom
        let (_, s) = split_bom(i);

        let mut state: SrtParserState = Emptyline; // expect emptyline or index

        // the `once("")` is there so no last entry gets ignored
        for (line_num, line) in s.lines().chain(once("")).enumerate() {
            state = match state {
                Emptyline => if line.trim().is_empty() { Emptyline } else { Index(Self::parse_index_line(line_num, line)?) },
                Index(index) => Timing(index, Self::parse_timespan_line(line_num, line)?),
                Timing(index, timespan) => Self::state_expect_dialog(line, &mut result, index, timespan, Vec::new()),
                Dialog(index, timespan, texts) => Self::state_expect_dialog(line, &mut result, index, timespan, texts),
            };
        }

        Ok(SrtFile { v: result })
    }

    fn state_expect_dialog(line: &str, result: &mut Vec<SrtLine>, index: i64, timespan: TimeSpan, mut texts: Vec<String>) -> SrtParserState {
        if line.trim().is_empty() {
            result.push(SrtLine {
                index: index,
                timespan: timespan,
                texts: texts,
            });
            SrtParserState::Emptyline
        } else {
            texts.push(line.trim().to_string());
            SrtParserState::Dialog(index, timespan, texts)
        }
    }

    /// Matches a line with a single index.
    fn parse_index_line(line_num: usize, s: &str) -> Result<i64> {
        s.trim()
         .parse::<i64>()
         .chain_err(|| ExpectedIndexLine(s.to_string()))
         .chain_err(|| ErrorAtLine(line_num))
    }

    /// Matches a `SubRip` timespan like "00:24:45,670 --> 00:24:45,680".
    fn parse_timespan_line(line_num: usize, line: &str) -> Result<TimeSpan> {

        /// Matches a `SubRip` timestamp like "00:24:45,670"
        let timestamp = |s| {
            (p(number_i64), char(':'), p(number_i64), char(':'), p(number_i64), char(','), p(number_i64))
                .map(|t| TimePoint::from_components(t.0, t.2, t.4, t.6))
                .parse_stream(s)
        };

        (skip_many(ws()), p(&timestamp), skip_many(ws()), string("-->"), skip_many(ws()), p(&timestamp), skip_many(ws()), eof())
            .map(|t| TimeSpan::new(t.1, t.5))
            .parse(line)
            .map(|x| x.0)
            .map_err(|_| Error::from(ExpectedTimestampLine(line.to_string())))
            .chain_err(|| ErrorAtLine(line_num))
    }
}

impl SubtitleFile for SrtFile {
    fn get_subtitle_entries(&self) -> SubtitleParserResult<Vec<SubtitleEntry>> {
        let timings = self.v
                          .iter()
                          .map(|line| SubtitleEntry::new(line.timespan, line.texts.iter().join("\n")))
                          .collect();

        Ok(timings)
    }

    fn update_subtitle_entries(&mut self, new_subtitle_entries: &[SubtitleEntry]) -> SubtitleParserResult<()> {
        assert_eq!(self.v.len(), new_subtitle_entries.len()); // required by specification of this function

        for (line_ref, new_entry_ref) in self.v.iter_mut().zip(new_subtitle_entries) {
            line_ref.timespan = new_entry_ref.timespan;
            if let Some(ref text) = new_entry_ref.line {
                line_ref.texts = text.lines().map(str::to_string).collect();
            }
        }

        Ok(())
    }

    fn to_data(&self) -> SubtitleParserResult<Vec<u8>> {
        let timepoint_to_str = |t: TimePoint| -> String {

            format!("{:02}:{:02}:{:02},{:03}",
                    t.hours(),
                    t.mins_comp(),
                    t.secs_comp(),
                    t.msecs_comp())
        };
        let line_to_str = |line: &SrtLine| -> String {
            format!("{}\n{} --> {}\n{}\n\n",
                    line.index,
                    timepoint_to_str(line.timespan.start),
                    timepoint_to_str(line.timespan.end),
                    line.texts.join("\n"))
        };

        Ok(self.v.iter().map(line_to_str).collect::<String>().into_bytes())
    }
}

impl SrtFile {
    /// Creates .srt file from scratch.
    pub fn create(v: Vec<(TimeSpan, String)>) -> SubtitleParserResult<SrtFile> {
        let file_parts = v.into_iter()
                          .enumerate()
                          .map(|(i, (ts, text))| {
            SrtLine {
                index: i as i64 + 1,
                timespan: ts,
                texts: text.lines().map(str::to_string).collect(),
            }
        })
                          .collect();

        Ok(SrtFile { v: file_parts })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_srt_test() {
        use timetypes::{TimePoint, TimeSpan};
        use SubtitleFile;

        let lines = vec![(TimeSpan::new(TimePoint::from_msecs(1500), TimePoint::from_msecs(3700)), "line1".to_string()),
                         (TimeSpan::new(TimePoint::from_msecs(4500), TimePoint::from_msecs(8700)), "line2".to_string())];
        let file = super::SrtFile::create(lines).unwrap();

        // generate file
        let data_string = String::from_utf8(file.to_data().unwrap()).unwrap();
        let expected = "1\n00:00:01,500 --> 00:00:03,700\nline1\n\n2\n00:00:04,500 --> 00:00:08,700\nline2\n\n".to_string();
        println!("\n{:?}\n{:?}", data_string, expected);
        assert_eq!(data_string, expected);
    }
}
// TODO: parser tests
