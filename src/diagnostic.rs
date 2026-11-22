use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStage {
    Lexing,
    Parsing,
    Semantic,
    Runtime,
    Bytecode,
}

impl fmt::Display for DiagnosticStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Lexing => "lexer",
            Self::Parsing => "parser",
            Self::Semantic => "semantic",
            Self::Runtime => "runtime",
            Self::Bytecode => "bytecode",
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

    pub fn semantic(message: impl Into<String>) -> Self {
        Self::new(DiagnosticStage::Semantic, message)
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(DiagnosticStage::Runtime, message)
    }

    /// Renders a diagnostic with the offending source line and a character-aware caret.
    pub fn render(&self, source: &str) -> String {
        let Some(location) = &self.location else {
            return self.to_string();
        };
        let Some(line_text) = source.lines().nth(location.line.saturating_sub(1)) else {
            return self.to_string();
        };
        let line_number = location.line;
        let gutter_width = line_number.to_string().len();
        let column = location.column.max(1);
        let caret_padding = " ".repeat(column.saturating_sub(1));
        format!(
            "{} error: {}\n --> {}:{}\n{:>width$} |\n{} | {}\n{:>width$} | {}^",
            self.stage,
            self.message,
            line_number,
            column,
            "",
            line_number,
            line_text,
            "",
            caret_padding,
            width = gutter_width
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_source_line_and_caret() {
        let diagnostic = Diagnostic::at(DiagnosticStage::Semantic, "unknown name", 2, 7);
        let rendered = diagnostic.render("let a = 1;\nprint(missing);");
        assert!(rendered.contains("2 | print(missing);"));
        assert!(rendered.contains("      ^"));
        assert!(rendered.contains(" --> 2:7"));
    }

    #[test]
    fn falls_back_when_source_line_is_unavailable() {
        let diagnostic = Diagnostic::at(DiagnosticStage::Parsing, "bad token", 4, 2);
        assert_eq!(diagnostic.render("one line"), diagnostic.to_string());
    }

    #[test]
    fn renders_locationless_diagnostics() {
        let diagnostic = Diagnostic::runtime("failed");
        assert_eq!(diagnostic.render(""), "runtime error: failed");
    }
}
