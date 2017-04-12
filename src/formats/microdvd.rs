// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use self::errors::*;
use self::errors::ErrorKind::*;
use {SubtitleEntry, SubtitleFile};

use combine::char::char;
use combine::combinator::{eof, many, parser as p, satisfy, sep_by};
use combine::primitives::Parser;
use errors::Result as SubtitleParserResult;
use formats::common::*;

use itertools::Itertools;
use std::borrow::Cow;
use std::collections::HashSet;

use std::collections::LinkedList;
use timetypes::{TimePoint, TimeSpan};

/// `.sub`(`MicroDVD`)-parser-specific errors
#[allow(missing_docs)]
pub mod errors {
    // see https://docs.rs/error-chain/0.8.1/error_chain/
    // this error type might be overkill, but that way it stays consistent with
    // the other parsers
    error_chain! {
        errors {
            ExpectedSubtitleLine(line: String) {
                display("expected subtittle line, found `{}`", line)
            }
            ErrorAtLine(line_num: usize) {
                display("parse error at line `{}`", line_num)
            }
        }
    }
}

/// Represents a formatting like "{y:i}" (display text in italics).
///
/// TODO: `MdvdFormatting` is a stub for the future where this enum holds specialized variants for different options.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum MdvdFormatting {
    /// A format option that is not directly supported.
    Unknown(String),
}

impl From<String> for MdvdFormatting {
    fn from(f: String) -> MdvdFormatting {
        MdvdFormatting::Unknown(Self::lowercase_first_char(&f))
    }
}

impl MdvdFormatting {
    /// Is this a single line formatting (e.g. `y:i`) or a multi-line formatting (e.g `Y:i`)?
    fn is_container_line_formatting(f: &str) -> bool {
        f.chars()
         .next()
         .and_then(|c| Some(c.is_uppercase()))
         .unwrap_or(false)
    }

    /// Applies `to_lowercase()` to first char, leaves the rest of the characters untouched.
    fn lowercase_first_char(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        }
    }

    /// Applies `to_uppercase()` to first char, leaves the rest of the characters untouched.
    fn uppercase_first_char(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    fn to_formatting_string_intern(&self) -> String {
        match *self {
            MdvdFormatting::Unknown(ref s) => s.clone(),
        }
    }

    /// Convert a `MdvdFormatting` to a string which can be used in `.sub` files.
    fn to_formatting_string(&self, multiline: bool) -> String {
        let s = self.to_formatting_string_intern();
        if multiline {
            Self::uppercase_first_char(&s)
        } else {
            Self::lowercase_first_char(&s)
        }
    }
}

#[derive(Debug, Clone)]
/// Represents a reconstructable `.sub`(`MicroDVD`) file.
pub struct MdvdFile {
    /// Number of frames per second of the accociated video (default 25)
    /// -> start/end frames can be coverted to timestamps
    fps: f64,

    /// all lines and multilines
    v: Vec<MdvdLine>,
}

/// Holds the description of a line like.
#[derive(Debug, Clone)]
struct MdvdLine {
    /// The start frame.
    start_frame: i64,

    /// The end frame.
    end_frame: i64,

    /// Formatting that affects all contained single lines.
    formatting: Vec<MdvdFormatting>,

    /// The (dialog) text of the line.
    text: String,
}

impl MdvdLine {
    fn to_subtitle_entry(&self, fps: f64) -> SubtitleEntry {
        SubtitleEntry {
            timespan: TimeSpan::new(TimePoint::from_msecs((self.start_frame as f64 * 1000.0 / fps) as i64),
                                    TimePoint::from_msecs((self.end_frame as f64 * 1000.0 / fps) as i64)),
            line: Some(self.text.clone()),
        }
    }
}

impl MdvdFile {
    /// Parse a `MicroDVD` `.sub` subtitle string to `MdvdFile`.
    pub fn parse(s: &str, fps: f64) -> SubtitleParserResult<MdvdFile> {
        let file_opt = Self::parse_file(s, fps);
        match file_opt {
            Ok(file) => Ok(file),
            Err(err) => Err(err.into()),
        }
    }
}

/// Implements parse functions.
impl MdvdFile {
    fn parse_file(i: &str, fps: f64) -> Result<MdvdFile> {
        let mut result: Vec<MdvdLine> = Vec::new();

        // remove utf-8 bom
        let (_, s) = split_bom(i);

        for (line_num, line) in s.lines().enumerate() {
            // a line looks like "{0}{25}{c:$0000ff}{y:b,u}{f:DeJaVuSans}{s:12}Hello!|{y:i}Hello2!" where
            // 0 and 25 are the start and end frames and the other information is the formatting.
            let mut lines: Vec<MdvdLine> = Self::parse_line(line_num, line)?;
            result.append(&mut lines);
        }

        Ok(MdvdFile {
               fps: fps,
               v: result,
           })
    }

    // Parses something like "{0}{25}{C:$0000ff}{y:b,u}{f:DeJaVuSans}{s:12}Hello!|{s:15}Hello2!"
    fn parse_line(line_num: usize, line: &str) -> Result<Vec<MdvdLine>> {

        /// Matches the regex "\{[^}]*\}"; parses something like "{some_info}".
        let sub_info = (char('{'), many(satisfy(|c| c != '}')), char('}'))
            .map(|(_, info, _): (_, String, _)| info)
            .expected("MicroDVD info");

        // Parse a single line (until separator '|'), something like "{C:$0000ff}{y:b,u}{f:DeJaVuSans}{s:12}Hello!"
        // Returns the a tuple of the multiline-formatting, the single-line formatting and the text of the single line.
        let single_line = (many(sub_info), many(satisfy(|c| c != '|')));

        // the '|' char splits single lines
        (char('{'), p(number_i64), char('}'), char('{'), p(number_i64), char('}'), sep_by(single_line, char('|')), eof())
            .map(|(_, start_frame, _, _, end_frame, _, fmt_strs_and_lines, ())| (start_frame, end_frame, fmt_strs_and_lines))
            .map(|(start_frame, end_frame, fmt_strs_and_lines): (i64, i64, Vec<(Vec<String>, String)>)| {
                Self::construct_mdvd_lines(start_frame, end_frame, fmt_strs_and_lines)
            })
            .parse(line)
            .map(|x| x.0)
            .map_err(|_| Error::from(ExpectedSubtitleLine(line.to_string())))
            .chain_err(|| ErrorAtLine(line_num))
    }

    /// Construct (possibly multiple) `MdvdLines` from a deconstructed file line
    /// like "{C:$0000ff}{y:b,u}{f:DeJaVuSans}{s:12}Hello!|{s:15}Hello2!".
    ///
    /// The third parameter is for the example
    /// like `[(["C:$0000ff", "y:b,u", "f:DeJaVuSans", "s:12"], "Hello!"), (["s:15"], "Hello2!")].
    fn construct_mdvd_lines(start_frame: i64, end_frame: i64, fmt_strs_and_lines: Vec<(Vec<String>, String)>) -> Vec<MdvdLine> {

        // saves all multiline formatting
        let mut cline_fmts: Vec<MdvdFormatting> = Vec::new();

        // convert the formatting strings to `MdvdFormatting` objects and split between multi-line and single-line formatting
        let fmts_and_lines =
            fmt_strs_and_lines.into_iter()
                              .map(|(fmts, text)| (Self::string_to_formatting(&mut cline_fmts, fmts), text))
                              .collect::<Vec<_>>();

        // now we also have all multi-line formattings in `cline_fmts`

        // finish creation of `MdvdLine`s
        fmts_and_lines.into_iter()
                      .map(|(sline_fmts, text)| {
            MdvdLine {
                start_frame: start_frame,
                end_frame: end_frame,
                text: text,
                formatting: cline_fmts.clone()
                                      .into_iter()
                                      .chain(sline_fmts.into_iter())
                                      .collect(),
            }
        })
                      .collect()
    }

    /// Convert `MicroDVD` formatting strings to `MdvdFormatting` objects.
    ///
    /// Move multiline formattings and single line formattings into different vectors.
    fn string_to_formatting(multiline_formatting: &mut Vec<MdvdFormatting>, fmts: Vec<String>) -> Vec<MdvdFormatting> {

        // split multiline-formatting (e.g "Y:b") and single-line formatting (e.g "y:b")
        let (cline_fmts_str, sline_fmts_str): (Vec<_>, Vec<_>) =
            fmts.into_iter()
                .partition(|fmt_str| MdvdFormatting::is_container_line_formatting(fmt_str));

        multiline_formatting.extend(&mut cline_fmts_str.into_iter().map(MdvdFormatting::from));
        sline_fmts_str.into_iter()
                      .map(MdvdFormatting::from)
                      .collect()
    }
}

impl SubtitleFile for MdvdFile {
    fn get_subtitle_entries(&self) -> SubtitleParserResult<Vec<SubtitleEntry>> {
        Ok(self.v
               .iter()
               .map(|line| line.to_subtitle_entry(self.fps))
               .collect())
    }

    fn update_subtitle_entries(&mut self, new_subtitle_entries: &[SubtitleEntry]) -> SubtitleParserResult<()> {
        assert_eq!(new_subtitle_entries.len(), self.v.len());

        let mut iter = new_subtitle_entries.iter().peekable();
        for line in &mut self.v {
            let peeked = iter.next().unwrap();

            line.start_frame = (peeked.timespan.start.secs_f64() * self.fps) as i64;
            line.end_frame = (peeked.timespan.end.secs_f64() * self.fps) as i64;

            if let Some(ref text) = peeked.line {
                line.text = text.clone();
            }
        }

        Ok(())
    }

    fn to_data(&self) -> SubtitleParserResult<Vec<u8>> {
        let mut sorted_list = self.v.clone();
        sorted_list.sort_by_key(|line| (line.start_frame, line.end_frame));

        let mut result: LinkedList<Cow<'static, str>> = LinkedList::new();

        for (gi, group_iter) in sorted_list.into_iter()
                                           .group_by(|line| (line.start_frame, line.end_frame))
                                           .into_iter()
                                           .enumerate() {
            if gi != 0 {
                result.push_back("\n".into());
            }

            let group: Vec<MdvdLine> = group_iter.1.collect();
            let group_len = group.len();

            let (start_frame, end_frame) = group_iter.0;
            let (formattings, texts): (Vec<HashSet<MdvdFormatting>>, Vec<String>) =
                group.into_iter()
                     .map(|line| (line.formatting.into_iter().collect(), line.text))
                     .unzip();

            // all single lines in the container line "cline" have the same start and end time
            //  -> the .sub file format let's them be on the same line with "{0}{1000}Text1|Text2"

            // find common formatting in all lines
            let common_formatting = if group_len == 1 {
                // if this "group" only has a single line, let's say that every formatting is individual
                HashSet::new()
            } else {
                formattings.iter()
                           .fold(None, |acc, set| match acc {
                    None => Some(set.clone()),
                    Some(acc_set) => Some(acc_set.intersection(set).cloned().collect()),
                })
                           .unwrap()
            };

            let individual_formattings = formattings.into_iter()
                                                    .map(|formatting| {
                                                             formatting.difference(&common_formatting)
                                                                       .cloned()
                                                                       .collect()
                                                         })
                                                    .collect::<Vec<HashSet<MdvdFormatting>>>();


            result.push_back("{".into());
            result.push_back(start_frame.to_string().into());
            result.push_back("}".into());

            result.push_back("{".into());
            result.push_back(end_frame.to_string().into());
            result.push_back("}".into());

            for formatting in &common_formatting {
                result.push_back("{".into());
                result.push_back(formatting.to_formatting_string(true).into());
                result.push_back("}".into());
            }

            for (i, (individual_formatting, text)) in
                individual_formattings.into_iter()
                                      .zip(texts.into_iter())
                                      .enumerate() {
                if i != 0 {
                    result.push_back("|".into());
                }

                for formatting in individual_formatting {
                    result.push_back("{".into());
                    result.push_back(formatting.to_formatting_string(false).into());
                    result.push_back("}".into());
                }

                result.push_back(text.into());
            }


            // ends "group-by-frametime"-loop
        }

        Ok(result.into_iter()
                 .map(|cow| cow.to_string())
                 .collect::<String>()
                 .into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SubtitleFile;

    /// Parse string with `MdvdFile`, and reencode it with `MdvdFile`.
    fn mdvd_reconstruct(s: &str) -> String {
        let file = MdvdFile::parse(s, 25.0).unwrap();
        let data = file.to_data().unwrap();
        String::from_utf8(data).unwrap()
    }

    /// Parse and re-construct `MicroDVD` files and test them against expected output.
    fn test_mdvd(input: &str, expected: &str) {
        // if we put the `input` into the parser, we expect a specific (cleaned-up) output
        assert_eq!(mdvd_reconstruct(input), expected);

        // if we reconstuct he cleaned-up output, we expect that nothing changes
        assert_eq!(mdvd_reconstruct(expected), expected);
    }

    #[test]
    fn mdvd_test_reconstruction() {
        // simple examples
        test_mdvd("{0}{25}Hello!", "{0}{25}Hello!");
        test_mdvd("{0}{25}{y:i}Hello!", "{0}{25}{y:i}Hello!");
        test_mdvd("{0}{25}{Y:i}Hello!", "{0}{25}{y:i}Hello!");
        test_mdvd("{0}{25}{Y:i}\n", "{0}{25}{y:i}");

        // cleanup formattings in a file
        test_mdvd("{0}{25}{y:i}Text1|{y:i}Text2", "{0}{25}{Y:i}Text1|Text2");
        test_mdvd("{0}{25}{y:i}Text1\n{0}{25}{y:i}Text2",
                  "{0}{25}{Y:i}Text1|Text2");
        test_mdvd("{0}{25}{y:i}{y:b}Text1\n{0}{25}{y:i}Text2",
                  "{0}{25}{Y:i}{y:b}Text1|Text2");
        test_mdvd("{0}{25}{y:i}{y:b}Text1\n{0}{25}{y:i}Text2",
                  "{0}{25}{Y:i}{y:b}Text1|Text2");

        // these can't be condensed, because the lines have different times
        test_mdvd("{0}{25}{y:i}Text1\n{0}{26}{y:i}Text2",
                  "{0}{25}{y:i}Text1\n{0}{26}{y:i}Text2");
    }
}
