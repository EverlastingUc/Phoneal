/// Primitive static types supported by Phoneal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhonealType { Integer, Text, Boolean }
impl std::fmt::Display for PhonealType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Integer => "integer", Self::Text => "text", Self::Boolean => "boolean" })
    }
}
