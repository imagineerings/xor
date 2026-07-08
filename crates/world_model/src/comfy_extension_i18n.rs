use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::SimExtensionId;

pub const SIM_EXTENSION_I18N_INVALID_BUNDLE_CODE: &str =
    "world_model.extension_i18n.invalid_bundle";
pub const SIM_EXTENSION_I18N_INVALID_LANGUAGE_CODE: &str =
    "world_model.extension_i18n.invalid_language";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimExtensionLocaleFileKind {
    Main,
    NodeDefs,
    Commands,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLocaleBundle {
    pub extension_id: SimExtensionId,
    pub language: String,
    pub files: BTreeMap<SimExtensionLocaleFileKind, serde_json::Value>,
}

impl SimExtensionLocaleBundle {
    pub fn new(extension_id: SimExtensionId, language: impl Into<String>) -> Self {
        Self {
            extension_id,
            language: language.into(),
            files: BTreeMap::new(),
        }
    }

    pub fn with_file(mut self, kind: SimExtensionLocaleFileKind, value: serde_json::Value) -> Self {
        self.files.insert(kind, value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLocaleLanguage {
    pub language: String,
    pub extension_ids: Vec<SimExtensionId>,
    pub files: BTreeMap<SimExtensionLocaleFileKind, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLocaleDiagnostic {
    pub code: String,
    pub extension_id: SimExtensionId,
    pub language: String,
    pub file_kind: Option<SimExtensionLocaleFileKind>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLocaleMergeReport {
    pub languages: Vec<SimExtensionLocaleLanguage>,
    pub diagnostics: Vec<SimExtensionLocaleDiagnostic>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionLocaleMerger;

impl SimExtensionLocaleMerger {
    pub fn new() -> Self {
        Self
    }

    pub fn merge(
        &self,
        bundles: impl IntoIterator<Item = SimExtensionLocaleBundle>,
    ) -> SimExtensionLocaleMergeReport {
        let mut report = SimExtensionLocaleMergeReport::default();
        let mut languages = BTreeMap::<String, SimExtensionLocaleLanguage>::new();

        for bundle in bundles {
            let language = bundle.language.trim().to_ascii_lowercase();
            if language.is_empty() {
                report.diagnostics.push(locale_diagnostic(
                    SIM_EXTENSION_I18N_INVALID_LANGUAGE_CODE,
                    bundle.extension_id,
                    bundle.language,
                    None,
                    "extension locale language must not be empty",
                ));
                continue;
            }

            let language_record =
                languages
                    .entry(language.clone())
                    .or_insert_with(|| SimExtensionLocaleLanguage {
                        language: language.clone(),
                        extension_ids: Vec::new(),
                        files: BTreeMap::new(),
                    });
            if !language_record
                .extension_ids
                .iter()
                .any(|extension_id| extension_id == &bundle.extension_id)
            {
                language_record
                    .extension_ids
                    .push(bundle.extension_id.clone());
            }

            for (file_kind, value) in bundle.files {
                if !value.is_object() {
                    report.diagnostics.push(locale_diagnostic(
                        SIM_EXTENSION_I18N_INVALID_BUNDLE_CODE,
                        bundle.extension_id.clone(),
                        language.clone(),
                        Some(file_kind),
                        "extension locale files must be JSON objects",
                    ));
                    continue;
                }

                language_record
                    .files
                    .entry(file_kind)
                    .and_modify(|existing| merge_json_objects(existing, &value))
                    .or_insert(value);
            }
        }

        report.languages = languages.into_values().collect();
        report
    }
}

fn merge_json_objects(target: &mut serde_json::Value, incoming: &serde_json::Value) {
    let (Some(target), Some(incoming)) = (target.as_object_mut(), incoming.as_object()) else {
        *target = incoming.clone();
        return;
    };

    for (key, value) in incoming {
        match (target.get_mut(key), value) {
            (Some(existing), serde_json::Value::Object(_)) if existing.is_object() => {
                merge_json_objects(existing, value);
            }
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

pub fn locale_languages(report: &SimExtensionLocaleMergeReport) -> BTreeSet<&str> {
    report
        .languages
        .iter()
        .map(|language| language.language.as_str())
        .collect()
}

fn locale_diagnostic(
    code: impl Into<String>,
    extension_id: SimExtensionId,
    language: impl Into<String>,
    file_kind: Option<SimExtensionLocaleFileKind>,
    message: impl Into<String>,
) -> SimExtensionLocaleDiagnostic {
    SimExtensionLocaleDiagnostic {
        code: code.into(),
        extension_id,
        language: language.into(),
        file_kind,
        message: message.into(),
    }
}
