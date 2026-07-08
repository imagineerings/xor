use crate::{DbThread, SharedThread};
use anyhow::{Context as _, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharedSessionData {
    pub version: String,
    pub session: SharedThread,
    pub metadata: ShareMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareMetadata {
    pub title: String,
    pub exported_at: DateTime<Utc>,
}

impl SharedSessionData {
    pub const VERSION: &'static str = "1.0.0";
    pub const LINK_PREFIX: &'static str = "sim://session/";

    pub fn from_db_thread(thread: &DbThread, exported_at: DateTime<Utc>) -> Self {
        Self {
            version: Self::VERSION.to_string(),
            session: SharedThread::from_db_thread(thread),
            metadata: ShareMetadata {
                title: thread.title.to_string(),
                exported_at,
            },
        }
    }

    pub fn to_db_thread(self) -> DbThread {
        self.session.to_db_thread()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        const COMPRESSION_LEVEL: i32 = 3;
        let json = serde_json::to_vec(self)?;
        Ok(zstd::encode_all(json.as_slice(), COMPRESSION_LEVEL)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let decompressed = zstd::decode_all(bytes)?;
        let data = serde_json::from_slice::<Self>(&decompressed)?;
        anyhow::ensure!(
            data.version == Self::VERSION,
            "unsupported shared session version {}",
            data.version
        );
        Ok(data)
    }

    pub fn to_share_code(&self) -> Result<String> {
        Ok(URL_SAFE_NO_PAD.encode(self.to_bytes()?))
    }

    pub fn from_share_code(code: &str) -> Result<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(code)
            .context("failed to decode shared session payload")?;
        Self::from_bytes(&bytes)
    }

    pub fn to_deeplink(&self) -> Result<String> {
        Ok(format!("{}{}", Self::LINK_PREFIX, self.to_share_code()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_thread() -> DbThread {
        DbThread {
            title: "Shared work".into(),
            messages: Vec::new(),
            updated_at: Utc.with_ymd_and_hms(2026, 7, 8, 12, 0, 0).unwrap(),
            detailed_summary: None,
            initial_project_snapshot: None,
            cumulative_token_usage: Default::default(),
            request_token_usage: Default::default(),
            model: None,
            profile: None,
            imported: false,
            subagent_context: None,
            speed: None,
            thinking_enabled: false,
            thinking_effort: None,
            draft_prompt: None,
            ui_scroll_position: None,
            sandboxed_terminal_temp_dir: None,
        }
    }

    #[test]
    fn shared_session_roundtrips_through_deeplink() {
        let exported_at = Utc.with_ymd_and_hms(2026, 7, 8, 12, 30, 0).unwrap();
        let data = SharedSessionData::from_db_thread(&make_thread(), exported_at);

        let link = data.to_deeplink().expect("encode shared session link");
        let code = link
            .strip_prefix(SharedSessionData::LINK_PREFIX)
            .expect("link has shared session prefix");
        let decoded = SharedSessionData::from_share_code(code).expect("decode shared session link");

        assert_eq!(decoded.version, SharedSessionData::VERSION);
        assert_eq!(decoded.metadata.title, "Shared work");
        assert_eq!(decoded.metadata.exported_at, exported_at);

        let imported = decoded.to_db_thread();
        assert!(imported.imported);
        assert_eq!(imported.title.as_ref(), "🔗 Shared work");
    }

    #[test]
    fn invalid_shared_session_code_returns_error() {
        assert!(SharedSessionData::from_share_code("not-base64").is_err());
    }
}
