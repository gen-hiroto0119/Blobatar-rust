pub(crate) struct AdvancedJsonDraft {
    last_synced: String,
}

impl AdvancedJsonDraft {
    pub(crate) fn new(last_synced: String) -> Self {
        Self { last_synced }
    }

    pub(crate) fn sync(&mut self, current: &str, canonical: String, force: bool) -> Option<String> {
        if !force && current != self.last_synced {
            return None;
        }
        if current == canonical {
            self.last_synced = canonical;
            return None;
        }
        self.last_synced = canonical.clone();
        Some(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::EditorState;

    fn canonical(state: &EditorState) -> String {
        serde_json::to_string(&state.settings.options.traits).unwrap()
    }

    #[test]
    fn review_dirty_advanced_json_survives_state_refresh() {
        let mut state = EditorState::default();
        let synced = canonical(&state);
        let mut draft = AdvancedJsonDraft::new(synced);
        state.pin("hue", 0.25);

        let invalid_draft = "{";
        assert_eq!(draft.sync(invalid_draft, canonical(&state), false), None);
        assert_eq!(draft.last_synced, "{}");
    }

    #[test]
    fn review_clean_advanced_json_syncs_to_canonical_state() {
        let mut state = EditorState::default();
        let synced = canonical(&state);
        let mut draft = AdvancedJsonDraft::new(synced.clone());
        state.pin("hue", 0.25);

        let next = canonical(&state);
        assert_eq!(draft.sync(&synced, next.clone(), false), Some(next.clone()));
        assert_eq!(draft.last_synced, next);
    }

    #[test]
    fn review_successful_apply_forces_canonical_json_sync() {
        let mut state = EditorState::default();
        let mut draft = AdvancedJsonDraft::new(canonical(&state));
        let applied = " { \"hue\" : 0.25 } ";
        state.apply_traits_json(applied).unwrap();

        let canonical = canonical(&state);
        assert_eq!(
            draft.sync(applied, canonical.clone(), true),
            Some(canonical.clone())
        );
        assert_eq!(draft.last_synced, canonical);
    }

    #[test]
    fn review_failed_apply_preserves_state_and_json_draft() {
        let mut state = EditorState::default();
        let original = serde_json::to_value(&state.settings).unwrap();
        let mut draft = AdvancedJsonDraft::new(canonical(&state));
        let invalid_draft = "{";

        assert!(state.apply_traits_json(invalid_draft).is_err());
        assert_eq!(serde_json::to_value(&state.settings).unwrap(), original);
        assert_eq!(draft.sync(invalid_draft, canonical(&state), false), None);
        assert_eq!(draft.last_synced, "{}");
    }
}
