mod parser;
mod types;

pub use parser::{detect_provider, extract_path_line_suffix, parse_remote_url};
pub use types::{ParseError, Provider, SrcuriTarget};
