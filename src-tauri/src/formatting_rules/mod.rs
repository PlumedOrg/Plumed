mod formatter;
mod helpers;
pub mod highlighter;
mod store;
pub mod types;

pub(crate) use formatter::apply_custom_rule;
#[cfg(test)]
pub use formatter::apply_formatting;
pub use formatter::apply_formatting_traced;
pub use store::{load, save};
pub use types::{BuiltInOptions, FormattingSettings, MatchMode};
