# logos-net-proxy

The single, **fail-closed** constructor of HTTP clients for the Logos wallet modules
(`eth_rpc_module`, `token_list_module`). It is the one chokepoint through which every
outbound request is built, so the wallet's privacy posture — *never send in the clear
when a proxy is required* — is enforced in one auditable place and is ready for Tor.

## API

```rust
pub struct ProxyConfig { pub proxy: Option<String>, pub proxy_required: bool, pub timeout_secs: u64 }
pub enum ProxyError { ProxyRequiredButUnset, ProxyUnusable(String), Build(String) }

/// The ONLY place a reqwest client is constructed in the wallet networking modules.
pub fn build_client(cfg: &ProxyConfig) -> Result<reqwest::blocking::Client, ProxyError>;
```

Rules enforced by `build_client`:

- `proxy_required == true` and no usable proxy ⇒ `Err(ProxyRequiredButUnset)` — it never
  falls back to a clear-net client.
- A proxy URL must use a supported scheme: `socks5h` (preferred — DNS resolves through the
  proxy, the right choice for Tor), `socks5`, `http`, or `https`.
- With no proxy set and none required, ambient environment proxies are disabled so the
  clear-net path is explicit and deterministic.

Consumers depend on this crate and have **no other way** to obtain a client; each asserts via
a unit test that `reqwest::Client::builder` appears only here.

## Build / test

```bash
cargo test
```

Uses `reqwest` with `rustls-tls` (pure-Rust TLS) + `socks`. Built with `rustls` rather than
`native-tls` to keep the link closure free of system OpenSSL.
