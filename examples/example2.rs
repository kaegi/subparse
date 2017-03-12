extern crate subparse;

use subparse::{SrtFile, SubtitleFile};
use subparse::timetypes::{TimePoint, TimeSpan};

fn main() {
    // example how to create a fresh .srt file
    let lines = vec![(TimeSpan::new(TimePoint::from_msecs(1500), TimePoint::from_msecs(3700)), "line1".to_string()),
                     (TimeSpan::new(TimePoint::from_msecs(4500), TimePoint::from_msecs(8700)), "line2".to_string())];
    let file = SrtFile::create(lines).unwrap();

    // generate file content
    let srt_string = String::from_utf8(file.to_data().unwrap()).unwrap();
    println!("{}", srt_string);
}
