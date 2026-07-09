use anyhow::{Context as _, Result, bail};
use sqlez::connection::Connection;
use std::cmp::Reverse;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATION: &str = "
    CREATE TABLE IF NOT EXISTS permission_decisions(
        tool_name TEXT NOT NULL,
        args_pattern TEXT NOT NULL,
        decision_type TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        expires_at INTEGER,
        PRIMARY KEY(tool_name, args_pattern)
    ) STRICT;
";

pub struct PermissionStore {
    connection: Connection,
}

impl PermissionStore {
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::new(Connection::open_file(
            path.as_ref().to_string_lossy().as_ref(),
        ))
    }

    pub fn open_memory(name: Option<&str>) -> Result<Self> {
        Self::new(Connection::open_memory(name))
    }

    pub fn new(connection: Connection) -> Result<Self> {
        connection
            .exec(MIGRATION)
            .context("failed to prepare permission store migration")?()
        .context("failed to migrate permission store")?;
        Ok(Self { connection })
    }

    pub fn record_decision(&self, decision: StoredDecision) -> Result<()> {
        self.connection
            .exec_bound::<(&str, &str, &str, i64, Option<i64>)>(
                "INSERT OR REPLACE INTO permission_decisions(
                    tool_name,
                    args_pattern,
                    decision_type,
                    created_at,
                    expires_at
                ) VALUES ((?), (?), (?), (?), (?))",
            )
            .context("failed to prepare permission decision upsert")?((
            &decision.tool_name,
            &decision.args_pattern,
            decision.decision_type.as_str(),
            decision.created_at,
            decision.expires_at,
        ))
        .context("failed to record permission decision")
    }

    pub fn get_decision(
        &self,
        tool_name: &str,
        args_pattern: &str,
    ) -> Result<Option<StoredDecision>> {
        self.connection
            .select_row_bound::<(&str, &str), StoredDecisionRow>(
                "SELECT tool_name, args_pattern, decision_type, created_at, expires_at
                 FROM permission_decisions
                 WHERE tool_name = (?) AND args_pattern = (?)",
            )
            .context("failed to prepare permission decision lookup")?((
            tool_name,
            args_pattern,
        ))
        .context("failed to read permission decision")?
        .map(StoredDecision::try_from)
        .transpose()
    }

    pub fn find_decision_for_args(
        &self,
        tool_name: &str,
        args: &str,
        now: i64,
    ) -> Result<Option<StoredDecision>> {
        let mut candidates = self.list_decisions_for_tool(tool_name)?;
        candidates.retain(|decision| {
            !decision.is_expired(now) && args_pattern_matches(&decision.args_pattern, args)
        });
        candidates.sort_by_key(|decision| Reverse(decision.args_pattern.len()));
        Ok(candidates.into_iter().next())
    }

    pub fn list_decisions_for_tool(&self, tool_name: &str) -> Result<Vec<StoredDecision>> {
        self.connection
            .select_bound::<&str, StoredDecisionRow>(
                "SELECT tool_name, args_pattern, decision_type, created_at, expires_at
                 FROM permission_decisions
                 WHERE tool_name = (?)
                 ORDER BY tool_name, args_pattern",
            )
            .context("failed to prepare permission decisions by tool query")?(tool_name)
        .context("failed to list permission decisions by tool")?
        .into_iter()
        .map(StoredDecision::try_from)
        .collect()
    }

    pub fn list_decisions(&self) -> Result<Vec<StoredDecision>> {
        self.connection
            .select::<StoredDecisionRow>(
                "SELECT tool_name, args_pattern, decision_type, created_at, expires_at
                 FROM permission_decisions
                 ORDER BY tool_name, args_pattern",
            )
            .context("failed to prepare permission decisions query")?()
        .context("failed to list permission decisions")?
        .into_iter()
        .map(StoredDecision::try_from)
        .collect()
    }

    pub fn delete_decision(&self, tool_name: &str, args_pattern: &str) -> Result<()> {
        self.connection
            .exec_bound::<(&str, &str)>(
                "DELETE FROM permission_decisions
                 WHERE tool_name = (?) AND args_pattern = (?)",
            )
            .context("failed to prepare permission decision delete")?((
            tool_name,
            args_pattern,
        ))
        .context("failed to delete permission decision")
    }

    pub fn clear(&self) -> Result<()> {
        self.connection
            .exec("DELETE FROM permission_decisions")
            .context("failed to prepare permission decision clear")?()
        .context("failed to clear permission decisions")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDecision {
    pub tool_name: String,
    pub args_pattern: String,
    pub decision_type: DecisionType,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl StoredDecision {
    pub fn new(
        tool_name: impl Into<String>,
        args_pattern: impl Into<String>,
        decision_type: DecisionType,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            args_pattern: args_pattern.into(),
            decision_type,
            created_at: now_unix_seconds(),
            expires_at,
        }
    }

    pub fn is_expired(&self, now: i64) -> bool {
        self.expires_at.is_some_and(|expires_at| expires_at <= now)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    AlwaysAllow,
    AlwaysDeny,
    AllowOnce,
    DenyOnce,
}

impl DecisionType {
    fn as_str(self) -> &'static str {
        match self {
            Self::AlwaysAllow => "always_allow",
            Self::AlwaysDeny => "always_deny",
            Self::AllowOnce => "allow_once",
            Self::DenyOnce => "deny_once",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "always_allow" => Ok(Self::AlwaysAllow),
            "always_deny" => Ok(Self::AlwaysDeny),
            "allow_once" => Ok(Self::AllowOnce),
            "deny_once" => Ok(Self::DenyOnce),
            _ => bail!("unknown permission decision type {value:?}"),
        }
    }
}

type StoredDecisionRow = (String, String, String, i64, Option<i64>);

impl TryFrom<StoredDecisionRow> for StoredDecision {
    type Error = anyhow::Error;

    fn try_from(row: StoredDecisionRow) -> Result<Self> {
        let (tool_name, args_pattern, decision_type, created_at, expires_at) = row;
        Ok(Self {
            tool_name,
            args_pattern,
            decision_type: DecisionType::from_str(&decision_type)?,
            created_at,
            expires_at,
        })
    }
}

fn args_pattern_matches(pattern: &str, args: &str) -> bool {
    if pattern == "*" || pattern == args {
        return true;
    }

    let mut remaining = args;
    let mut parts = pattern.split('*').peekable();
    let starts_with_wildcard = pattern.starts_with('*');
    let ends_with_wildcard = pattern.ends_with('*');

    if let Some(first) = parts.next()
        && !first.is_empty()
    {
        if !starts_with_wildcard && !remaining.starts_with(first) {
            return false;
        }
        if let Some(rest) = remaining.get(first.len()..) {
            remaining = rest;
        } else {
            return false;
        }
    }

    while let Some(part) = parts.next() {
        if part.is_empty() {
            continue;
        }

        let Some(index) = remaining.find(part) else {
            return false;
        };
        let next_index = index + part.len();
        if let Some(rest) = remaining.get(next_index..) {
            remaining = rest;
        } else {
            return false;
        }

        if parts.peek().is_none() && !ends_with_wildcard && !remaining.is_empty() {
            return false;
        }
    }

    true
}

fn now_unix_seconds() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_reads_and_replaces_decisions() {
        let store = PermissionStore::open_memory(Some("records_reads_and_replaces_decisions"))
            .expect("store should open");
        let decision = StoredDecision {
            tool_name: "terminal".into(),
            args_pattern: "git status".into(),
            decision_type: DecisionType::AlwaysAllow,
            created_at: 10,
            expires_at: None,
        };

        store
            .record_decision(decision)
            .expect("decision should write");
        assert_eq!(
            store
                .get_decision("terminal", "git status")
                .expect("decision should read")
                .expect("decision should exist")
                .decision_type,
            DecisionType::AlwaysAllow
        );

        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "git status".into(),
                decision_type: DecisionType::AlwaysDeny,
                created_at: 11,
                expires_at: Some(20),
            })
            .expect("replacement should write");
        let stored = store
            .get_decision("terminal", "git status")
            .expect("decision should read")
            .expect("decision should exist");
        assert_eq!(stored.decision_type, DecisionType::AlwaysDeny);
        assert_eq!(stored.expires_at, Some(20));
    }

    #[test]
    fn lists_deletes_and_clears_decisions() {
        let store = PermissionStore::open_memory(Some("lists_deletes_and_clears_decisions"))
            .expect("store should open");

        store
            .record_decision(StoredDecision::new(
                "terminal",
                "cargo test",
                DecisionType::AllowOnce,
                None,
            ))
            .expect("decision should write");
        store
            .record_decision(StoredDecision::new(
                "editor",
                "*",
                DecisionType::AlwaysDeny,
                None,
            ))
            .expect("decision should write");

        assert_eq!(store.list_decisions().expect("list should read").len(), 2);
        assert_eq!(
            store
                .list_decisions_for_tool("terminal")
                .expect("tool list should read")
                .len(),
            1
        );

        store
            .delete_decision("terminal", "cargo test")
            .expect("delete should succeed");
        assert!(
            store
                .get_decision("terminal", "cargo test")
                .expect("decision should read")
                .is_none()
        );

        store.clear().expect("clear should succeed");
        assert!(store.list_decisions().expect("list should read").is_empty());
    }

    #[test]
    fn finds_unexpired_matching_decision() {
        let store = PermissionStore::open_memory(Some("finds_unexpired_matching_decision"))
            .expect("store should open");
        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "cargo *".into(),
                decision_type: DecisionType::AlwaysAllow,
                created_at: 1,
                expires_at: Some(100),
            })
            .expect("decision should write");
        store
            .record_decision(StoredDecision {
                tool_name: "terminal".into(),
                args_pattern: "cargo test --ignored".into(),
                decision_type: DecisionType::AlwaysDeny,
                created_at: 1,
                expires_at: Some(5),
            })
            .expect("decision should write");

        let decision = store
            .find_decision_for_args("terminal", "cargo test --ignored", 10)
            .expect("decision lookup should succeed")
            .expect("unexpired wildcard should match");

        assert_eq!(decision.args_pattern, "cargo *");
        assert_eq!(decision.decision_type, DecisionType::AlwaysAllow);
    }

    #[test]
    fn persists_decisions_to_file() {
        let tempdir = tempfile::tempdir().expect("tempdir should create");
        let path = tempdir.path().join("permissions.sqlite");

        {
            let store = PermissionStore::open_file(&path).expect("store should open");
            store
                .record_decision(StoredDecision::new(
                    "terminal",
                    "npm test",
                    DecisionType::AlwaysAllow,
                    None,
                ))
                .expect("decision should write");
        }

        let reopened = PermissionStore::open_file(&path).expect("store should reopen");
        let decision = reopened
            .get_decision("terminal", "npm test")
            .expect("decision should read")
            .expect("decision should persist");

        assert_eq!(decision.decision_type, DecisionType::AlwaysAllow);
    }
}
