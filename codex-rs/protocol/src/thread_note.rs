const THREAD_NOTE_PURPOSE_PREFIX: &str = "Назначение:";
const THREAD_NOTE_COMPETENCIES_PREFIX: &str = "Компетенции:";
const THREAD_NOTE_SECTION_SEPARATOR: &str = " | ";

pub const MAX_THREAD_NOTE_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredThreadNote {
    pub purpose: String,
    pub competencies: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadNoteError {
    TooLong {
        max_chars: usize,
        actual_chars: usize,
    },
    DuplicateCompetencies,
}

impl std::fmt::Display for ThreadNoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLong {
                max_chars,
                actual_chars,
            } => write!(
                f,
                "thread_note is too long: {actual_chars} characters, maximum is {max_chars}"
            ),
            Self::DuplicateCompetencies => write!(
                f,
                "thread_note competencies were provided twice; pass competencies either inside canonical thread_note or in thread_note_competencies"
            ),
        }
    }
}

impl std::error::Error for ThreadNoteError {}

pub fn normalize_thread_note(note: Option<&str>) -> Result<Option<String>, ThreadNoteError> {
    normalize_thread_note_parts(note, None)
}

pub fn normalize_thread_note_parts(
    note: Option<&str>,
    competencies: Option<&str>,
) -> Result<Option<String>, ThreadNoteError> {
    let Some(note) = note else {
        let Some(competencies) = normalize_optional_text(competencies) else {
            return Ok(None);
        };
        return render_normalized_thread_note(StructuredThreadNote {
            purpose: String::new(),
            competencies,
        });
    };
    let Some(structured) = parse_thread_note(note) else {
        let Some(competencies) = normalize_optional_text(competencies) else {
            return Ok(None);
        };
        return render_normalized_thread_note(StructuredThreadNote {
            purpose: String::new(),
            competencies,
        });
    };
    let structured = match normalize_optional_text(competencies) {
        Some(_) if !structured.competencies.is_empty() => {
            return Err(ThreadNoteError::DuplicateCompetencies);
        }
        Some(competencies) => StructuredThreadNote {
            competencies,
            ..structured
        },
        None => structured,
    };
    render_normalized_thread_note(structured)
}

fn render_normalized_thread_note(
    structured: StructuredThreadNote,
) -> Result<Option<String>, ThreadNoteError> {
    if structured.purpose.is_empty() && structured.competencies.is_empty() {
        return Ok(None);
    }
    let rendered = render_thread_note(&structured);
    let actual_chars = rendered.chars().count();
    if actual_chars > MAX_THREAD_NOTE_CHARS {
        return Err(ThreadNoteError::TooLong {
            max_chars: MAX_THREAD_NOTE_CHARS,
            actual_chars,
        });
    }
    Ok(Some(rendered))
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
}

pub fn parse_thread_note(note: &str) -> Option<StructuredThreadNote> {
    let trimmed = note.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((purpose_section, competencies_section)) = trimmed.split_once('|')
        && let Some(purpose) = purpose_section
            .trim()
            .strip_prefix(THREAD_NOTE_PURPOSE_PREFIX)
        && let Some(competencies) = competencies_section
            .trim()
            .strip_prefix(THREAD_NOTE_COMPETENCIES_PREFIX)
    {
        return Some(StructuredThreadNote {
            purpose: collapse_whitespace(purpose),
            competencies: collapse_whitespace(competencies),
        });
    }

    if let Some(body) = trimmed.strip_prefix(THREAD_NOTE_PURPOSE_PREFIX)
        && !trimmed.contains('|')
    {
        let (purpose, competencies) = if let Some((purpose_section, competencies_section)) =
            body.trim().split_once(THREAD_NOTE_COMPETENCIES_PREFIX)
        {
            (purpose_section, competencies_section)
        } else {
            (body, "")
        };
        return Some(StructuredThreadNote {
            purpose: collapse_whitespace(purpose),
            competencies: collapse_whitespace(competencies),
        });
    }

    Some(StructuredThreadNote {
        purpose: collapse_whitespace(trimmed),
        competencies: String::new(),
    })
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn render_thread_note(note: &StructuredThreadNote) -> String {
    let purpose = if note.purpose.is_empty() {
        String::new()
    } else {
        format!(" {}", note.purpose)
    };
    if note.competencies.is_empty() {
        format!(
            "{THREAD_NOTE_PURPOSE_PREFIX}{purpose}{THREAD_NOTE_SECTION_SEPARATOR}{THREAD_NOTE_COMPETENCIES_PREFIX}"
        )
    } else {
        format!(
            "{THREAD_NOTE_PURPOSE_PREFIX}{purpose}{THREAD_NOTE_SECTION_SEPARATOR}{THREAD_NOTE_COMPETENCIES_PREFIX} {}",
            note.competencies
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_thread_note_trims_collapses_whitespace_and_rejects_empty() {
        assert_eq!(normalize_thread_note(None), Ok(None));
        assert_eq!(normalize_thread_note(Some("   ")), Ok(None));
        assert_eq!(
            normalize_thread_note(Some("  keep   this\n\tclose  ")),
            Ok(Some(
                "Назначение: keep this close | Компетенции:".to_string()
            ))
        );
    }

    #[test]
    fn parse_thread_note_supports_structured_values() {
        assert_eq!(
            parse_thread_note("Назначение: Исследователь | Компетенции: docs/fork/features"),
            Some(StructuredThreadNote {
                purpose: "Исследователь".to_string(),
                competencies: "docs/fork/features".to_string(),
            })
        );
        assert_eq!(
            parse_thread_note("Назначение: Исследователь Компетенции: docs/fork/features"),
            Some(StructuredThreadNote {
                purpose: "Исследователь".to_string(),
                competencies: "docs/fork/features".to_string(),
            })
        );
    }

    #[test]
    fn parse_thread_note_treats_non_canonical_structured_text_as_plain() {
        assert_eq!(
            normalize_thread_note(Some("Purpose: audit | Skills: docs")),
            Ok(Some(
                "Назначение: Purpose: audit | Skills: docs | Компетенции:".to_string()
            ))
        );
        assert_eq!(
            normalize_thread_note(Some(r#"{"purpose":"audit"}"#)),
            Ok(Some(
                r#"Назначение: {"purpose":"audit"} | Компетенции:"#.to_string()
            ))
        );
    }

    #[test]
    fn normalize_thread_note_clears_empty_canonical_structured_values() {
        assert_eq!(
            normalize_thread_note(Some("Назначение:    | Компетенции:  ")),
            Ok(None)
        );
        assert_eq!(
            normalize_thread_note(Some("Назначение: | Компетенции: docs")),
            Ok(Some("Назначение: | Компетенции: docs".to_string()))
        );
    }

    #[test]
    fn normalize_thread_note_parts_combines_purpose_and_competencies() {
        assert_eq!(
            normalize_thread_note_parts(Some("Тесты"), Some("fmt, lint")),
            Ok(Some(
                "Назначение: Тесты | Компетенции: fmt, lint".to_string()
            ))
        );
        assert_eq!(
            normalize_thread_note_parts(None, Some("fmt, lint")),
            Ok(Some("Назначение: | Компетенции: fmt, lint".to_string()))
        );
        assert_eq!(
            normalize_thread_note_parts(Some("   "), Some("  ")),
            Ok(None)
        );
    }

    #[test]
    fn normalize_thread_note_parts_rejects_duplicate_competencies() {
        assert_eq!(
            normalize_thread_note_parts(Some("Назначение: Тесты | Компетенции: fmt"), Some("lint")),
            Err(ThreadNoteError::DuplicateCompetencies)
        );
    }

    #[test]
    fn normalize_thread_note_rejects_over_limit() {
        let long = "x".repeat(MAX_THREAD_NOTE_CHARS + 1);
        assert!(matches!(
            normalize_thread_note(Some(&long)),
            Err(ThreadNoteError::TooLong { .. })
        ));
    }
}
