#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SimMode {
    Focus,
    #[default]
    Balanced,
    Creative,
}

impl From<settings::SimModeContent> for SimMode {
    fn from(value: settings::SimModeContent) -> Self {
        match value {
            settings::SimModeContent::Focus => Self::Focus,
            settings::SimModeContent::Balanced => Self::Balanced,
            settings::SimModeContent::Creative => Self::Creative,
        }
    }
}

impl SimMode {
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
            SimMode::from(settings::SimModeContent::Focus),
            SimMode::Focus
        );
        assert_eq!(
            SimMode::from(settings::SimModeContent::Balanced),
            SimMode::Balanced
        );
        assert_eq!(
            SimMode::from(settings::SimModeContent::Creative),
            SimMode::Creative
        );
    }

    #[test]
    fn modes_have_distinct_prompt_instructions() {
        assert_ne!(
            SimMode::Focus.instructions(),
            SimMode::Balanced.instructions()
        );
        assert_ne!(
            SimMode::Creative.instructions(),
            SimMode::Balanced.instructions()
        );
    }
}
