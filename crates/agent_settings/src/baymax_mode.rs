#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BaymaxMode {
    Focus,
    #[default]
    Balanced,
    Creative,
}

impl From<settings::BaymaxModeContent> for BaymaxMode {
    fn from(value: settings::BaymaxModeContent) -> Self {
        match value {
            settings::BaymaxModeContent::Focus => Self::Focus,
            settings::BaymaxModeContent::Balanced => Self::Balanced,
            settings::BaymaxModeContent::Creative => Self::Creative,
        }
    }
}

impl BaymaxMode {
    pub fn instructions(&self) -> &'static str {
        match self {
            Self::Focus => {
                "Prioritize concise, low-risk changes. Ask before broadening scope, and prefer the smallest correct implementation."
            }
            Self::Balanced => {
                "Balance speed and completeness. Make pragmatic implementation choices while preserving codebase patterns."
            }
            Self::Creative => {
                "Explore broader solution space when useful. Still keep changes grounded in the requested task and repository patterns."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_settings_content_modes() {
        assert_eq!(
            BaymaxMode::from(settings::BaymaxModeContent::Focus),
            BaymaxMode::Focus
        );
        assert_eq!(
            BaymaxMode::from(settings::BaymaxModeContent::Balanced),
            BaymaxMode::Balanced
        );
        assert_eq!(
            BaymaxMode::from(settings::BaymaxModeContent::Creative),
            BaymaxMode::Creative
        );
    }

    #[test]
    fn modes_have_distinct_prompt_instructions() {
        assert_ne!(
            BaymaxMode::Focus.instructions(),
            BaymaxMode::Balanced.instructions()
        );
        assert_ne!(
            BaymaxMode::Creative.instructions(),
            BaymaxMode::Balanced.instructions()
        );
    }
}
