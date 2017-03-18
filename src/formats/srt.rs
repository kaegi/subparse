// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


use {ParseSubtitleString, SubtitleEntry, SubtitleFile};
use errors::Result as SubtitleParserResult;
use formats::common::*;
use timetypes::{TimePoint, TimeSpan};
use self::errors::ErrorKind::*;
use self::errors::*;

use std;

use combine::char::{char, string};
use combine::combinator::{eof, many, parser as p};
use combine::primitives::{ParseError, ParseResult, Parser, Stream};

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
            LineParserError(line_num: usize, msg: String) {
                display("parse error at line `{}` because of `{}`", line_num, msg)
            }
            SrtParseError(msg: String) {
                description(msg)
            }
        }
    }
}

/// This makes creating a vector with file parts much nicer, shorter and more readable.
trait ExtendWithSrtFilePart {
    fn filler(self, s: String) -> Self;
    fn index(self, i: i64) -> Self;
    fn dialog(self, s: String) -> Self;
    fn begin(self, t: TimePoint) -> Self;
    fn end(self, t: TimePoint) -> Self;
}

impl ExtendWithSrtFilePart for Vec<SrtFilePart> {
    fn filler(mut self, s: String) -> Self {
        self.push(SrtFilePart::Filler(s));
        self
    }
    fn index(mut self, i: i64) -> Self {
        self.push(SrtFilePart::Index(i));
        self
    }
    fn dialog(mut self, s: String) -> Self {
        self.push(SrtFilePart::Dialog(s));
        self
    }
    fn begin(mut self, t: TimePoint) -> Self {
        self.push(SrtFilePart::TimespanBegin(t));
        self
    }
    fn end(mut self, t: TimePoint) -> Self {
        self.push(SrtFilePart::TimespanEnd(t));
        self
    }
}


/// The parsing works as a finite state machine. These are the states in it.
enum SrtParserState {
    // emptyline or index follows
    Emptyline,

    /// timing line follows
    Index,

    /// dialog or emptyline follows
    Timing,

    /// emptyline follows
    Dialog,
}

/// The whole .srt file will be split into semantic segments (index, text,
/// timepan information) and this enum provides the information which
/// information a segment holds.
#[derive(Debug, Clone)]
enum SrtFilePart {
    /// Indices, spaces, empty lines, etc.
    Filler(String),

    /// The beginnig timestamp of a timespan
    TimespanBegin(TimePoint),

    /// The ending timestamp of a timespan
    TimespanEnd(TimePoint),

    /// The index, which determines the order of all subtitle blocks
    Index(i64),

    /// The dialog text
    Dialog(String),
}

#[derive(Debug, Clone)]
/// Represents a reconstructable `.srt` file.
pub struct SrtFile {
    v: Vec<SrtFilePart>,
}

impl ParseSubtitleString for SrtFile {
    fn parse_from_string(s: String) -> SubtitleParserResult<SrtFile> {
        let file_opt = Self::parse_file(s.as_str());
        match file_opt {
            Ok(file) => Ok(SrtFile::new(file)),
            Err(err) => Err(err.into()),
        }
    }
}

/// Implements parse functions.
impl SrtFile {
    fn parse_file(i: &str) -> Result<Vec<SrtFilePart>> {
        let mut result: Vec<SrtFilePart> = Vec::new();

        // remove utf-8 bom
        let (bom, s) = split_bom(i);
        result = result.filler(bom.to_string());


        let mut state: SrtParserState = SrtParserState::Emptyline; // expect emptyline or index
        let lines_with_newl: Vec<(String, String)> = get_lines_non_destructive(s)
            .map_err(|(line_num, err_str)| LineParserError(line_num, err_str))?;

        for (line_num, (line, newl)) in lines_with_newl.into_iter().enumerate() {
            state = match state {
                SrtParserState::Emptyline => Self::next_state_from_emptyline(&mut result, line_num, line)?,
                SrtParserState::Index => Self::next_state_from_index(&mut result, line_num, line)?,
                SrtParserState::Timing | SrtParserState::Dialog => Self::next_state_from_timing_or_dialog(&mut result, line_num, line)?,
            };

            // we also want to preserve the line break
            result.push(SrtFilePart::Filler(newl));
        }

        Ok(result)
    }

    fn next_state_from_emptyline(result: &mut Vec<SrtFilePart>, line_num: usize, line: String) -> Result<SrtParserState> {
        if line.trim().is_empty() {
            result.push(SrtFilePart::Filler(line));
            Ok(SrtParserState::Emptyline)
        } else {
            result.append(&mut Self::parse_index_line(line_num, line.as_str())?);
            Ok(SrtParserState::Index)
        }
    }


    fn next_state_from_index(result: &mut Vec<SrtFilePart>, line_num: usize, line: String) -> Result<SrtParserState> {
        result.append(&mut Self::parse_timestamp_line(line_num, line.as_str())?);
        Ok(SrtParserState::Timing)
    }

    fn next_state_from_timing_or_dialog(result: &mut Vec<SrtFilePart>, _: usize, line: String) -> Result<SrtParserState> {
        if line.trim().is_empty() {
            result.push(SrtFilePart::Filler(line));
            Ok(SrtParserState::Emptyline)
        } else {
            result.push(SrtFilePart::Dialog(line));
            Ok(SrtParserState::Dialog)
        }
    }

    /// Matches a line with a single index.
    fn parse_index_line(line_num: usize, s: &str) -> Result<Vec<SrtFilePart>> {
        Self::handle_error((many(ws()), p(number_i64), many(ws()), eof())
                               .map(|(ws1, num, ws2, ()): (_, _, _, ())| Vec::new().filler(ws1).index(num).filler(ws2))
                               .expected("SubRip index")
                               .parse(s),
                           line_num,
                           || ExpectedIndexLine(s.to_string()).into())
    }

    /// Convert a result/error from the combine library to the srt parser error.
    fn handle_error<T, F>(r: std::result::Result<(T, &str), ParseError<&str>>, line_num: usize, err_func: F) -> Result<T>
        where F: FnOnce() -> Error
    {
        r.map(|(v, _)| v)
         .map_err(|_| err_func())
         .chain_err(|| Error::from(ErrorAtLine(line_num)))
    }


    /// Matches a `SubRip` timestamp like "00:24:45,670"
    fn parse_timestamp<I>(input: I) -> ParseResult<TimePoint, I>
        where I: Stream<Item = char>
    {
        (p(number_i64), char(':'), p(number_i64), char(':'), p(number_i64), char(','), p(number_i64))
            .map(|t| TimePoint::from_components(t.0, t.2, t.4, t.6))
            .parse_stream(input)
    }


    /// Matches a `SubRip` timespan like "00:24:45,670 --> 00:24:45,680".
    fn parse_timespan<I>(input: I) -> ParseResult<Vec<SrtFilePart>, I>
        where I: Stream<Item = char>
    {
        (many(ws()), p(Self::parse_timestamp), many(ws()), string("-->"), many(ws()), p(Self::parse_timestamp), many(ws()), eof())
            .map(|t| Vec::new().filler(t.0).begin(t.1).filler(t.2).filler(t.3.to_string()).filler(t.4).end(t.5).filler(t.6))
            .parse_stream(input)
    }

    /// Matches a `SubRip` timespan line like "00:24:45,670 --> 00:24:45,680".
    fn parse_timestamp_line(line_num: usize, s: &str) -> Result<Vec<SrtFilePart>> {
        Self::handle_error(p(Self::parse_timespan).parse(s),
                           line_num,
                           || ExpectedTimestampLine(s.to_string()).into())
    }
}

impl SubtitleFile for SrtFile {
    fn get_subtitle_entries(&self) -> SubtitleParserResult<Vec<SubtitleEntry>> {
        // it's unfortunate we have to clone the file before using
        // `get_subtitle_entries_mut()`, but otherwise we'd have to copy the`
        // `get_subtitle_entries_mut()` and create a non-mut-reference version
        // of it (much code duplication); I think a `clone` in this
        // not-time-critical code is acceptable, and after HKT become
        // available, this can be solved much nicer.
        let mut new_file = self.clone();
        let timings = new_file.get_subtitle_entries_mut()
                              .into_iter()
                              .map(|(&mut start, &mut end, text)| SubtitleEntry::new(TimeSpan::new(start, end), text.clone()))
                              .collect();

        Ok(timings)
    }

    fn update_subtitle_entries(&mut self, new_subtitle_entries: &[SubtitleEntry]) -> SubtitleParserResult<()> {
        let subtitle_entries = self.get_subtitle_entries_mut();
        assert_eq!(subtitle_entries.len(), new_subtitle_entries.len()); // required by specification of this function

        for ((start_ref, end_ref, text_ref), new_entry_ref) in subtitle_entries.into_iter().zip(new_subtitle_entries) {
            *start_ref = new_entry_ref.timespan.start;
            *end_ref = new_entry_ref.timespan.end;
            if let Some(ref text) = new_entry_ref.line {
                *text_ref = text.clone();
            }
        }

        Ok(())
    }

    fn to_data(&self) -> SubtitleParserResult<Vec<u8>> {
        let closure = &mut |part: &SrtFilePart| {
            use self::SrtFilePart::*;
            match *part {
                Filler(ref t) | Dialog(ref t) => t.clone(),
                Index(i) => i.to_string(),
                TimespanBegin(t) | TimespanEnd(t) => {
                    format!("{:02}:{:02}:{:02},{:03}",
                            t.hours(),
                            t.mins_comp(),
                            t.secs_comp(),
                            t.msecs_comp())
                }
            }
        };

        Ok(self.v.iter().map(closure).collect::<String>().into_bytes())
    }
}

impl SrtFile {
    fn new(v: Vec<SrtFilePart>) -> SrtFile {
        // merges multiple fillers after another
        let new_file_parts = dedup_string_parts(v, |part: &mut SrtFilePart| {
            match *part {
                SrtFilePart::Filler(ref mut text) => Some(text),
                _ => None,
            }
        });

        SrtFile { v: Self::squash_dialog_lines(new_file_parts) }
    }

    /// Squashes Dialog lines and Fillers (e.g. newlines) between dialog into a single Dialog file part.
    fn squash_dialog_lines(v: Vec<SrtFilePart>) -> Vec<SrtFilePart> {
        // merges multiple dialog lines after another (shouldn't exist with current parser, but
        // you can never be sure :) )
        let v2 = dedup_string_parts(v, |part: &mut SrtFilePart| {
            match *part {
                SrtFilePart::Dialog(ref mut text) => Some(text),
                _ => None,
            }
        });

        // Merge file part sequence "Dialog -> Filler -> Dialog" to a single "Dialog" block; we
        // pop the last two elementsand if they are "Dialog" and "Fillers"
        // and the current part is a "Dialog", we squash the lines and put the new "Dialog" in
        // the queue.
        let mut result = Vec::new();
        for file_part in v2 {
            if result.len() < 2 {
                result.push(file_part);
            } else {
                // squash current file part and queue?
                let prev = result.pop().unwrap();
                let preprev = result.pop().unwrap();

                let mut replace_with_text = None;
                if let SrtFilePart::Dialog(ref d2) = file_part {
                    if let SrtFilePart::Dialog(ref d1) = preprev {
                        if let SrtFilePart::Filler(ref f) = prev {
                            replace_with_text = Some(format!("{}{}{}", d1, f, d2));
                        }
                    }
                }

                if let Some(text) = replace_with_text {
                    result.push(SrtFilePart::Dialog(text));
                } else {
                    // no squash -> re-push the old file parts and the new
                    result.push(preprev);
                    result.push(prev);
                    result.push(file_part);
                }
            }
        }

        result
    }


    /// Creates .srt file from scratch.
    pub fn create(v: Vec<(TimeSpan, String)>) -> SubtitleParserResult<SrtFile> {
        let mut file_parts = Vec::new();
        for (i, (ts, line)) in v.into_iter().enumerate() {
            file_parts.push(SrtFilePart::Filler(format!("{}\n", i + 1)));
            file_parts.push(SrtFilePart::TimespanBegin(ts.start));
            file_parts.push(SrtFilePart::Filler(" --> ".to_string()));
            file_parts.push(SrtFilePart::TimespanEnd(ts.end));
            file_parts.push(SrtFilePart::Filler("\n".to_string()));
            file_parts.push(SrtFilePart::Dialog(line));
            file_parts.push(SrtFilePart::Filler("\n\n".to_string()));
        }

        Ok(SrtFile { v: file_parts })
    }


    // TODO: implement a single version that takes both `&mut` and `&` (dependent on HKT).
    // XXX: this can be abstracted over with the same function in `SsaFile`
    /// This function filters out all start times and end times, and returns them ordered
    /// (="(start, end)") so they can be easily read or written to.
    fn get_subtitle_entries_mut<'a>(&'a mut self) -> Vec<(&'a mut TimePoint, &'a mut TimePoint, &'a mut String)> {

        let mut startpoint_buffer: Option<&'a mut TimePoint> = None;
        let mut endpoint_buffer: Option<&'a mut TimePoint> = None;
        let result = {
            // satisfy the borrow checker, so next_state is released
            let closure = &mut |part: &'a mut SrtFilePart| -> Option<(&'a mut TimePoint, &'a mut TimePoint, &'a mut String)> {
                use self::SrtFilePart::*;
                match *part {
                    TimespanBegin(ref mut start) => {
                        assert_eq!(startpoint_buffer, None); // parser should have ensured that no two consecutive SRT start times exist
                        startpoint_buffer = Some(start);
                        None
                    }
                    TimespanEnd(ref mut end) => {
                        assert_eq!(endpoint_buffer, None); // parser should have ensured that no two consecutive SRT end times exist
                        endpoint_buffer = Some(end);
                        None
                    }
                    Dialog(ref mut text) => {
                        // reset the timepoint buffers
                        let snatched_startpoint_buffer = startpoint_buffer.take();
                        let snatched_endpoint_buffer = endpoint_buffer.take();

                        let start = snatched_startpoint_buffer.expect("SRT parser should have ensured that every line has a startpoint");
                        let end = snatched_endpoint_buffer.expect("SRT parser should have ensured that every line has a endpoint");

                        Some((start, end, text))
                    }

                    Filler(_) | Index(_) => None,
                }
            };
            self.v.iter_mut().filter_map(closure).collect()
        };

        // every timespan should now consist of a beginning and a end
        assert_eq!(startpoint_buffer, None);
        assert_eq!(endpoint_buffer, None);
        result
    }
}

#[cfg(test)]
mod tests {
    #[allow(unsafe_code)]
    fn parse_srt(s: String) {
        use {ParseSubtitleString, SubtitleFile};

        // reconstruct file
        let srt_file = super::SrtFile::parse_from_string(s.clone()).unwrap();
        let s2 = srt_file.to_data().unwrap();
        assert_eq!(s,
                   unsafe { ::std::str::from_utf8_unchecked(&s2) }.to_string());
    }

    #[test]
    #[allow(unsafe_code)]
    fn parse_srt_test() {
        // test "normal" file
        let s1 = "1\n".to_string() + "00:00:31,915 --> 00:00:35,903\r\n" + "♬～\r\n" + "\n" + "2\n" + "00:00:35,903 --> 00:00:44,912\n" + "♬～";
        parse_srt(s1.clone());
        parse_srt(s1.clone() + "\n   ");
        parse_srt(s1.clone() + "\n   \r\n");

        // test "empty" files
        parse_srt("".to_string());
        parse_srt("\n".to_string());

        // test file without dialog
        let s3 = "1\n".to_string() + "00:00:31,915 --> 00:00:35,903\r\n" + "\r\n" + "   \n" + "2\n" + "00:00:35,903 --> 00:00:44,912\n";
        parse_srt(s3.clone());
        parse_srt(s3.clone() + "\n   ");
        parse_srt(s3.clone() + "\n   \r\n");

        // the bom should be preserved
        parse_srt(unsafe { ::std::str::from_utf8_unchecked(&[0xFE, 0xFF]) }.to_string());
    }

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
