use color_eyre::Result;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;

/// Build a wordlist from the schedule for customising Azure speech-to-text.
///
/// Take the talk titles and speaker names and directly include them in the
/// list. For the descriptions, pull out capitalised words and words not
/// present in `/usr/share/dict/words` to select the likely technical terms.
pub fn make_wordlist_file<Writer>(
    file: Writer,
    event: &crate::Event,
) -> Result<()>
where
    Writer: Write,
{
    // Talk titles and speakers
    let mut titles = BTreeSet::new();
    let mut speakers = BTreeSet::new();

    let dict = load_usr_share_dict_words()?;
    let mut words = BTreeSet::new();

    for session in event.iter_sessions() {
        titles.insert(session.title.as_str());
        speakers.extend(session.speakers.iter().map(String::as_str));

        for word in session.description.split_whitespace() {
            // Remove leading and trailing punctuation
            let word = word.trim_matches(|chr: char| !chr.is_alphanumeric());
            if word.len() > 2 && !dict.contains(&word.to_lowercase()) {
                words.insert(word);
            }
        }
    }

    let mut file = BufWriter::new(file);
    for entry in titles.into_iter().chain(speakers).chain(words) {
        writeln!(&mut file, "{entry}")?;
    }
    Ok(())
}

fn load_usr_share_dict_words() -> std::io::Result<HashSet<String>> {
    let file = std::fs::File::open("/usr/share/dict/words")?;
    let file = BufReader::new(file);
    file.lines().map(|line| Ok(line?.to_lowercase())).collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::formats::{pretalx::Pretalx, sessionize_json::SessionizeJson};
    use std::io::Cursor;

    const BRISTOL: &str = include_str!("../test-data/bristol-2026.json");
    const BELFAST_JSON: &str = include_str!("../test-data/belfast.json");

    #[test]
    fn test_wordlist_bristol() {
        let bristol = serde_json::from_str::<Pretalx>(BRISTOL).unwrap();
        let event = crate::Event::try_from(bristol).unwrap();

        let mut file = Vec::<u8>::new();
        let cur = Cursor::new(&mut file);
        make_wordlist_file(cur, &event).unwrap();
        let out = String::from_utf8(file).unwrap();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_wordlist_belfast() {
        let belfast =
            serde_json::from_str::<SessionizeJson>(BELFAST_JSON).unwrap();
        let event = crate::Event::try_from(belfast).unwrap();

        let mut file = Vec::<u8>::new();
        let cur = Cursor::new(&mut file);
        make_wordlist_file(cur, &event).unwrap();
        let out = String::from_utf8(file).unwrap();
        insta::assert_snapshot!(out);
    }
}
