use color_eyre::Result;

use crate::event::EventIndex;

// pub mod ical;
// pub mod ontime;
pub mod show_file;

pub trait EventFormatter {
    fn format(&self, event: EventIndex, output_dir: &std::path::Path) -> Result<()>;
}

pub const DEFAULT_FORMATTERS: &[&dyn EventFormatter] = &[
    &show_file::ShowFile,
];
