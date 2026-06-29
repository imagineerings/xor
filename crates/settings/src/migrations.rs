use anyhow::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsMigration {
    original_text: String,
    migrated_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsMigrationStatus {
    UpToDate,
    NeedsMigration(SettingsMigration),
    Failed { error: String },
}

impl SettingsMigration {
    pub fn original_text(&self) -> &str {
        &self.original_text
    }

    pub fn migrated_text(&self) -> &str {
        &self.migrated_text
    }

    pub fn into_migrated_text(self) -> String {
        self.migrated_text
    }

    pub fn rollback_text(&self) -> &str {
        &self.original_text
    }
}

pub fn detect_settings_migration(text: &str) -> SettingsMigrationStatus {
    match migrate_settings_config(text) {
        Ok(Some(migration)) => SettingsMigrationStatus::NeedsMigration(migration),
        Ok(None) => SettingsMigrationStatus::UpToDate,
        Err(error) => SettingsMigrationStatus::Failed {
            error: error.to_string(),
        },
    }
}

pub fn migrate_settings_config(text: &str) -> Result<Option<SettingsMigration>> {
    migrator::migrate_settings(text).map(|migrated_text| {
        migrated_text.map(|migrated_text| SettingsMigration {
            original_text: text.to_string(),
            migrated_text,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_up_to_date_settings() {
        assert_eq!(
            detect_settings_migration(r#"{ "theme": "One Dark" }"#),
            SettingsMigrationStatus::UpToDate
        );
    }

    #[test]
    fn detects_and_applies_migration() {
        let SettingsMigrationStatus::NeedsMigration(migration) =
            detect_settings_migration(r#"{ "hide_mouse": "on_typing_and_movement" }"#)
        else {
            panic!("expected settings migration");
        };

        assert_eq!(
            migration.migrated_text(),
            r#"{ "hide_mouse": "on_typing_and_action" }"#
        );
        assert_eq!(
            migration.rollback_text(),
            r#"{ "hide_mouse": "on_typing_and_movement" }"#
        );
    }

    #[test]
    fn migration_retains_original_for_rollback() {
        let migration = migrate_settings_config(
            r#"{
  "hide_mouse": "on_typing_and_movement"
}"#,
        )
        .expect("settings migration should run")
        .expect("settings should need migration");

        assert_eq!(
            migration.rollback_text(),
            r#"{
  "hide_mouse": "on_typing_and_movement"
}"#
        );
        assert_ne!(migration.rollback_text(), migration.migrated_text());
    }
}
