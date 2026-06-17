//! `logos-net-proxy` — the single, fail-closed constructor of HTTP clients for the
//! Logos wallet modules.
//!
//! Every outbound request in `eth_rpc_module` and `token_list_module` is built
//! through [`build_client`], so the privacy posture — *never send in the clear
//! when a proxy is required* — is enforced in **one place** and cannot be
//! bypassed by a forgotten setter. There is deliberately no other way to obtain a
//! `reqwest::Client` in those modules; a unit test in each consumer asserts that
//! `reqwest::Client::builder` appears only here.

use std::time::Duration;
use thiserror::Error;

/// Outbound network policy for the client we are about to build.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProxyConfig {
    /// Proxy URL, e.g. `socks5h://127.0.0.1:9050`. `socks5h` resolves DNS through
    /// the proxy (the privacy-preferred scheme for Tor). `None` means no proxy is
    /// configured.
    pub proxy: Option<String>,
    /// When `true`, a request MUST traverse a proxy. If `proxy` is `None` or
    /// unusable, [`build_client`] **fails closed** (returns `Err`) instead of
    /// building a clear-net client.
    pub proxy_required: bool,
    /// Total per-request timeout in seconds. `0` leaves reqwest's default.
    pub timeout_secs: u64,
}

impl ProxyConfig {
    pub fn new(proxy: Option<String>, proxy_required: bool, timeout_secs: u64) -> Self {
        Self { proxy, proxy_required, timeout_secs }
    }
}

/// Why a client could not be constructed.
#[derive(Debug, Error)]
pub enum ProxyError {
    /// Policy requires a proxy but none is configured. The fail-closed case.
    #[error("proxy required but none configured (fail-closed: refusing to send in the clear)")]
    ProxyRequiredButUnset,
    /// A proxy URL was supplied but is not a valid/supported proxy.
    #[error("proxy URL is invalid or unsupported: {0}")]
    ProxyUnusable(String),
    /// reqwest itself failed to build the client.
    #[error("failed to build HTTP client: {0}")]
    Build(String),
}

/// Schemes we accept for a proxy URL. `socks5h` is preferred (remote DNS);
/// `socks5`, `http`, and `https` are also permitted.
fn validate_proxy_url(p: &str) -> Result<(), ProxyError> {
    let parsed = url::Url::parse(p).map_err(|e| ProxyError::ProxyUnusable(format!("{p}: {e}")))?;
    match parsed.scheme() {
        "socks5h" | "socks5" | "http" | "https" => Ok(()),
        other => Err(ProxyError::ProxyUnusable(format!("unsupported proxy scheme: {other}"))),
    }
}

/// Build a blocking [`reqwest::blocking::Client`] honoring `cfg`. This is the ONLY
/// place a client is constructed in the wallet networking modules.
///
/// Rules:
/// - `proxy_required == true` and no usable `proxy` ⇒ [`ProxyError::ProxyRequiredButUnset`].
/// - `proxy` present but unparseable/unsupported ⇒ [`ProxyError::ProxyUnusable`].
/// - otherwise a client is built; a configured proxy applies to all requests, and
///   when no proxy is set (and none is required) environment proxies are disabled
///   so behavior is deterministic (no accidental `HTTP_PROXY` pickup).
pub fn build_client(cfg: &ProxyConfig) -> Result<reqwest::blocking::Client, ProxyError> {
    let mut builder = reqwest::blocking::Client::builder();

    let has_proxy = cfg.proxy.as_deref().is_some_and(|p| !p.trim().is_empty());
    if has_proxy {
        let p = cfg.proxy.as_deref().unwrap().trim();
        validate_proxy_url(p)?;
        let proxy = reqwest::Proxy::all(p).map_err(|e| ProxyError::ProxyUnusable(e.to_string()))?;
        builder = builder.proxy(proxy);
    } else {
        if cfg.proxy_required {
            return Err(ProxyError::ProxyRequiredButUnset);
        }
        // No proxy required and none set: disable any ambient env proxy so the
        // clear-net path is explicit and deterministic.
        builder = builder.no_proxy();
    }

    if cfg.timeout_secs > 0 {
        builder = builder.timeout(Duration::from_secs(cfg.timeout_secs));
    }

    builder.build().map_err(|e| ProxyError::Build(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_closed_when_required_and_unset() {
        let cfg = ProxyConfig::new(None, true, 30);
        assert!(matches!(build_client(&cfg), Err(ProxyError::ProxyRequiredButUnset)));
    }

    #[test]
    fn fail_closed_when_required_and_blank() {
        let cfg = ProxyConfig::new(Some("   ".into()), true, 30);
        assert!(matches!(build_client(&cfg), Err(ProxyError::ProxyRequiredButUnset)));
    }

    #[test]
    fn ok_when_not_required_and_unset() {
        let cfg = ProxyConfig::new(None, false, 30);
        assert!(build_client(&cfg).is_ok());
    }

    #[test]
    fn ok_with_socks5h_proxy() {
        // Build only — the connection is lazy, so no proxy needs to be running.
        let cfg = ProxyConfig::new(Some("socks5h://127.0.0.1:9050".into()), true, 30);
        assert!(build_client(&cfg).is_ok());
    }

    #[test]
    fn rejects_unsupported_scheme() {
        let cfg = ProxyConfig::new(Some("ftp://127.0.0.1:21".into()), true, 30);
        assert!(matches!(build_client(&cfg), Err(ProxyError::ProxyUnusable(_))));
    }

    #[test]
    fn rejects_garbage_proxy() {
        let cfg = ProxyConfig::new(Some("not a url".into()), true, 30);
        assert!(matches!(build_client(&cfg), Err(ProxyError::ProxyUnusable(_))));
    }
}
