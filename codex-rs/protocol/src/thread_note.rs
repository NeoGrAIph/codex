const THREAD_NOTE_PURPOSE_PREFIX: &str = "Назначение:";
const THREAD_NOTE_COMPETENCIES_PREFIX: &str = "Компетенции:";
const THREAD_NOTE_SECTION_SEPARATOR: &str = " | ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredThreadNote {
    pub purpose: String,
    pub competencies: String,
}

pub fn normalize_thread_note(note: Option<&str>) -> Option<String> {
    note.and_then(|note| {
        let structured = parse_thread_note(note)?;
        if structured.purpose.is_empty() {
            None
        } else {
            Some(render_thread_note(&structured))
        }
    })
}

pub fn parse_thread_note(note: &str) -> Option<StructuredThreadNote> {
    let trimmed = note.trim();
    if trimmed.is_empty() {
        return None;
    }

    let looks_structured = trimmed.contains('|')
        || trimmed.starts_with(THREAD_NOTE_PURPOSE_PREFIX)
        || trimmed.contains(THREAD_NOTE_COMPETENCIES_PREFIX);

    let structured = trimmed
        .split_once('|')
        .and_then(|(purpose_section, competencies_section)| {
            let purpose = purpose_section
                .trim()
                .strip_prefix(THREAD_NOTE_PURPOSE_PREFIX)?
                .trim();
            let competencies = competencies_section
                .trim()
                .strip_prefix(THREAD_NOTE_COMPETENCIES_PREFIX)?
                .trim();
            if purpose.is_empty() {
                return None;
            }
            Some(StructuredThreadNote {
                purpose: purpose.to_string(),
                competencies: competencies.to_string(),
            })
        });

    structured.or_else(|| {
        (!looks_structured).then_some(StructuredThreadNote {
            purpose: trimmed.to_string(),
            competencies: String::new(),
        })
    })
}

pub fn render_thread_note(note: &StructuredThreadNote) -> String {
    if note.competencies.is_empty() {
        format!(
            "{THREAD_NOTE_PURPOSE_PREFIX} {}{THREAD_NOTE_SECTION_SEPARATOR}{THREAD_NOTE_COMPETENCIES_PREFIX}",
            note.purpose
        )
    } else {
        format!(
            "{THREAD_NOTE_PURPOSE_PREFIX} {}{THREAD_NOTE_SECTION_SEPARATOR}{THREAD_NOTE_COMPETENCIES_PREFIX} {}",
            note.purpose, note.competencies
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_thread_note_trims_and_rejects_empty() {
        assert_eq!(normalize_thread_note(None), None);
        assert_eq!(normalize_thread_note(Some("   ")), None);
        assert_eq!(
            normalize_thread_note(Some("  keep this  ")),
            Some("Назначение: keep this | Компетенции:".to_string())
        );
    }

    #[test]
    fn normalize_thread_note_preserves_structured_form_and_spacing() {
        assert_eq!(
            normalize_thread_note(Some(
                "  Назначение: Исследователь репозитория| Компетенции: docs/fork/features; AGENTS.md  "
            )),
            Some(
                "Назначение: Исследователь репозитория | Компетенции: docs/fork/features; AGENTS.md"
                    .to_string()
            )
        );
    }

    #[test]
    fn normalize_thread_note_requires_non_empty_purpose() {
        assert_eq!(
            normalize_thread_note(Some("Назначение:   | Компетенции: docs/fork/features")),
            None
        );
    }

    #[test]
    fn parse_thread_note_supports_legacy_and_structured_values() {
        assert_eq!(
            parse_thread_note("Repository researcher"),
            Some(StructuredThreadNote {
                purpose: "Repository researcher".to_string(),
                competencies: String::new(),
            })
        );
        assert_eq!(
            parse_thread_note(
                "Назначение: Исследователь | Компетенции: docs/fork/features; AGENTS.md"
            ),
            Some(StructuredThreadNote {
                purpose: "Исследователь".to_string(),
                competencies: "docs/fork/features; AGENTS.md".to_string(),
            })
        );
    }
}
