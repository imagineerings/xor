use axum::http::HeaderMap;

use crate::{ApiError, AuthConfig};

pub const API_KEY_HEADER: &str = "x-api-key";

pub fn require_auth(headers: &HeaderMap, auth: &AuthConfig) -> Result<(), ApiError> {
    match auth {
        AuthConfig::None => Ok(()),
        AuthConfig::ApiKey { key } => {
            let provided = headers
                .get(API_KEY_HEADER)
                .and_then(|value| value.to_str().ok());
            if provided == Some(key.as_str()) {
                Ok(())
            } else {
                Err(ApiError::Unauthorized)
            }
        }
        AuthConfig::BearerToken { token } => {
            let expected = format!("Bearer {token}");
            let provided = headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok());
            if provided == Some(expected.as_str()) {
                Ok(())
            } else {
                Err(ApiError::Unauthorized)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn accepts_valid_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, HeaderValue::from_static("secret"));

        assert!(
            require_auth(
                &headers,
                &AuthConfig::ApiKey {
                    key: "secret".into()
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_missing_bearer_token() {
        let headers = HeaderMap::new();
        let error = require_auth(
            &headers,
            &AuthConfig::BearerToken {
                token: "secret".into(),
            },
        )
        .unwrap_err();

        assert!(matches!(error, ApiError::Unauthorized));
    }
}
