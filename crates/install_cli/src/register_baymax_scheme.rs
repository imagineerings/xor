use client::BAYMAX_URL_SCHEME;
use gpui::{AsyncApp, actions};

actions!(
    cli,
    [
        /// Registers the baymax:// URL scheme handler.
        RegisterBaymaxScheme
    ]
);

pub async fn register_baymax_scheme(cx: &AsyncApp) -> anyhow::Result<()> {
    cx.update(|cx| cx.register_url_scheme(BAYMAX_URL_SCHEME)).await
}
