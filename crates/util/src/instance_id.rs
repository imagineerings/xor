use anyhow::{Context as _, Result};
use std::fmt;
use std::path::Path;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InstanceId(Uuid);

impl InstanceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self> {
        Ok(Self(Uuid::parse_str(value.trim()).with_context(|| {
            format!("invalid instance id '{}'", value.trim())
        })?))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for InstanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

pub fn load_or_create(path: impl AsRef<Path>) -> Result<InstanceId> {
    let path = path.as_ref();
    match std::fs::read_to_string(path) {
        Ok(contents) => InstanceId::parse(&contents)
            .with_context(|| format!("failed to read instance id from {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let instance_id = InstanceId::new();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create instance id directory {}",
                        parent.display()
                    )
                })?;
            }
            std::fs::write(path, format!("{instance_id}\n"))
                .with_context(|| format!("failed to write instance id to {}", path.display()))?;
            Ok(instance_id)
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to read instance id from {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_or_create_persists_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("nested").join("instance-id");

        let first = load_or_create(&path).unwrap();
        let second = load_or_create(&path).unwrap();

        assert_eq!(first, second);
        assert_eq!(std::fs::read_to_string(path).unwrap(), format!("{first}\n"));
    }

    #[test]
    fn test_load_or_create_rejects_invalid_existing_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("instance-id");
        std::fs::write(&path, "not a uuid").unwrap();

        let error = load_or_create(&path).unwrap_err();

        assert!(error.to_string().contains("failed to read instance id"));
    }

    #[test]
    fn test_parse_trims_whitespace() {
        let id = InstanceId::new();

        assert_eq!(InstanceId::parse(&format!("  {id}\n")).unwrap(), id);
    }
}
