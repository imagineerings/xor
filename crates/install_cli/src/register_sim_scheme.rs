use client::SIM_URL_SCHEME;
use gpui::{AsyncApp, actions};

actions!(
    cli,
    [
        /// Registers the sim:// URL scheme handler.
        RegisterSimScheme
    ]
);

pub async fn register_sim_scheme(cx: &AsyncApp) -> anyhow::Result<()> {
    cx.update(|cx| cx.register_url_scheme(SIM_URL_SCHEME))
        .await
}
