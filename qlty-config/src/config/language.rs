use super::smells::Smells;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[allow(unused)]
pub struct Language {
    #[serde(default = "_default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub test_syntax_patterns: Vec<String>,

    pub smells: Option<Smells>,
}

const fn _default_true() -> bool {
    true
}
