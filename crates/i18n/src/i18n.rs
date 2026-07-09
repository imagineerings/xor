use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_LOCALE: &str = "en-US";

pub mod keys {
    pub const AGENT_PANEL_RECIPES: &str = "agent_panel.recipes";
    pub const AGENT_PANEL_DIAGNOSTICS: &str = "agent_panel.diagnostics";
    pub const AGENT_PANEL_SELECTED_AGENT: &str = "agent_panel.selected_agent";
    pub const RECIPE_BROWSER_SEARCH_PLACEHOLDER: &str = "recipe_browser.search_placeholder";
    pub const RECIPE_BROWSER_NO_RECIPES: &str = "recipe_browser.no_recipes";
    pub const RECIPE_BROWSER_RUN: &str = "recipe_browser.run";
    pub const DIAGNOSTICS_TITLE: &str = "diagnostics.title";
    pub const DIAGNOSTICS_RERUN: &str = "diagnostics.rerun";
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageSettings {
    pub locale: String,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            locale: DEFAULT_LOCALE.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationFile {
    pub locale: String,
    #[serde(default)]
    pub messages: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationCatalog {
    locale: String,
    messages: HashMap<String, String>,
    source_path: Option<PathBuf>,
}

impl TranslationCatalog {
    pub fn new(locale: impl Into<String>, messages: HashMap<String, String>) -> Self {
        Self {
            locale: locale.into(),
            messages,
            source_path: None,
        }
    }

    pub fn from_json_str(json: &str) -> Result<Self> {
        let file = serde_json::from_str::<TranslationFile>(json)?;
        Ok(Self::new(file.locale, file.messages))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let json = fs::read_to_string(path)
            .with_context(|| format!("loading translation file {}", path.display()))?;
        let mut catalog = Self::from_json_str(&json)
            .with_context(|| format!("parsing translation file {}", path.display()))?;
        catalog.source_path = Some(path.to_path_buf());
        Ok(catalog)
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    pub fn message(&self, key: &str) -> Option<&str> {
        self.messages.get(key).map(String::as_str)
    }
}

#[derive(Clone, Debug)]
pub struct I18n {
    active_locale: String,
    fallback_locale: String,
    catalogs: HashMap<String, TranslationCatalog>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new(DEFAULT_LOCALE)
    }
}

impl I18n {
    pub fn new(fallback_locale: impl Into<String>) -> Self {
        let fallback_locale = fallback_locale.into();
        Self {
            active_locale: fallback_locale.clone(),
            fallback_locale,
            catalogs: HashMap::default(),
        }
    }

    pub fn active_locale(&self) -> &str {
        &self.active_locale
    }

    pub fn fallback_locale(&self) -> &str {
        &self.fallback_locale
    }

    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.active_locale = locale.into();
    }

    pub fn add_catalog(&mut self, catalog: TranslationCatalog) {
        self.catalogs.insert(catalog.locale.clone(), catalog);
    }

    pub fn load_catalog_file(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.add_catalog(TranslationCatalog::load(path)?);
        Ok(())
    }

    pub fn load_catalogs_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<usize> {
        let dir = dir.as_ref();
        let mut loaded = 0;

        for entry in fs::read_dir(dir)
            .with_context(|| format!("loading translations from {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            self.load_catalog_file(&path)?;
            loaded += 1;
        }

        Ok(loaded)
    }

    pub fn translate<'a>(&'a self, key: &'a str) -> Cow<'a, str> {
        self.lookup(key)
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(key))
    }

    pub fn translate_with_args<'a>(&'a self, key: &'a str, args: &[(&str, &str)]) -> Cow<'a, str> {
        let message = self.translate(key);
        if args.is_empty() {
            return message;
        }

        let mut rendered = message.into_owned();
        for (name, value) in args {
            rendered = rendered.replace(&format!("{{{name}}}"), value);
        }
        Cow::Owned(rendered)
    }

    fn lookup(&self, key: &str) -> Option<&str> {
        self.catalogs
            .get(&self.active_locale)
            .and_then(|catalog| catalog.message(key))
            .or_else(|| {
                self.catalogs
                    .get(&self.fallback_locale)
                    .and_then(|catalog| catalog.message(key))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(locale: &str, pairs: &[(&str, &str)]) -> TranslationCatalog {
        TranslationCatalog::new(
            locale,
            pairs
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        )
    }

    #[test]
    fn translates_active_locale_with_fallback() {
        let mut i18n = I18n::new("en-US");
        i18n.add_catalog(catalog(
            "en-US",
            &[
                (keys::AGENT_PANEL_RECIPES, "Recipes"),
                (keys::AGENT_PANEL_DIAGNOSTICS, "Diagnostics"),
            ],
        ));
        i18n.add_catalog(catalog("es", &[(keys::AGENT_PANEL_RECIPES, "Recetas")]));
        i18n.set_locale("es");

        assert_eq!(i18n.translate(keys::AGENT_PANEL_RECIPES), "Recetas");
        assert_eq!(i18n.translate(keys::AGENT_PANEL_DIAGNOSTICS), "Diagnostics");
        assert_eq!(i18n.translate("missing.key"), "missing.key");
    }

    #[test]
    fn interpolates_named_arguments() {
        let mut i18n = I18n::new("en-US");
        i18n.add_catalog(catalog("en-US", &[("welcome", "Welcome, {name}.")]));

        assert_eq!(
            i18n.translate_with_args("welcome", &[("name", "Ava")]),
            "Welcome, Ava."
        );
    }

    #[test]
    fn loads_json_catalogs_from_directory() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let catalog_path = temp_dir.path().join("en-US.json");
        fs::write(
            &catalog_path,
            r#"{"locale":"en-US","messages":{"agent_panel.recipes":"Recipes"}}"#,
        )
        .expect("write catalog");

        let mut i18n = I18n::new("en-US");
        assert_eq!(i18n.load_catalogs_from_dir(temp_dir.path()).unwrap(), 1);
        assert_eq!(i18n.translate(keys::AGENT_PANEL_RECIPES), "Recipes");
    }
}
