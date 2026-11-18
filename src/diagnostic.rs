use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStage {
    Lexing,
    Parsing,
    Runtime,
}

impl fmt::Display for DiagnosticStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Lexing => "lexer",
            Self::Parsing => "parser",
            Self::Runtime => "runtime",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub stage: DiagnosticStage,
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl Diagnostic {
    pub fn new(stage: DiagnosticStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
            location: None,
        }
    }

    pub fn at(
        stage: DiagnosticStage,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            stage,
            message: message.into(),
            location: Some(SourceLocation { line, column }),
        }
    }

    pub fn lexical(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::at(DiagnosticStage::Lexing, message, line, column)
    }

    pub fn parsing(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::at(DiagnosticStage::Parsing, message, line, column)
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(DiagnosticStage::Runtime, message)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} error: {}", self.stage, self.message)?;
        if let Some(location) = &self.location {
            write!(formatter, " at {}:{}", location.line, location.column)?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}
