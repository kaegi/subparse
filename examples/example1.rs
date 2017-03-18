extern crate subparse;

use subparse::timetypes::TimeDelta;
use subparse::SubtitleEntry;
use subparse::{get_subtitle_format_by_ending_err, parse_str};

/// This function reads the content of a file to a `String`.
fn read_file(path: &str) -> String {
    use std::io::Read;
    let mut file = std::fs::File::open(path).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn main() {
    // your setup goes here
    let path = "path/your_example_file.ssa";
    let file_content: String = read_file(path); // your own load routine

    // parse the file
    let format = get_subtitle_format_by_ending_err(path).expect("unknown format");
    let mut subtitle_file = parse_str(format, &file_content).expect("parser error");
    let mut subtitle_entries: Vec<SubtitleEntry> = subtitle_file.get_subtitle_entries().expect("unexpected error");

    // shift all subtitle entries by 1 minute and append "subparse" to each subtitle line
    for subtitle_entry in &mut subtitle_entries {
        subtitle_entry.timespan += TimeDelta::from_mins(1);

        // image based subtitles like .idx (VobSub) don't have text, so
        // a text is optional
        if let Some(ref mut line_ref) = subtitle_entry.line {
            line_ref.push_str("subparse");
        }
    }

    // update the entries in the subtitle file
    subtitle_file.update_subtitle_entries(&subtitle_entries).expect("unexpected error");

    // print the corrected file to stdout
    let data: Vec<u8> = subtitle_file.to_data().expect("unexpected errror");
    let data_string = String::from_utf8(data).expect("UTF-8 conversion error");
    println!("{}", data_string);
}
