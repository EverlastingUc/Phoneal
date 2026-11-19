/// Static types supported by the current Phoneal expression system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhonealType {
    Integer,
    String,
}

impl std::fmt::Display for PhonealType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Integer => "integer",
            Self::String => "string",
        })
    }
}
