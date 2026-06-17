# Proving the Fail-Closed Invariant of logos-net-proxy

`logos-net-proxy` is the wallet's **single outbound chokepoint**: the only
place an HTTP client is constructed. It is **fail-closed** — `build_client`
returns an error when a proxy is *required* but unset or unusable, so no code
path can ever yield a cleartext client behind the user's back. It is built on
`reqwest` with `rustls-tls` (pure-Rust at link) and `socks` for
`socks5h://` (remote DNS through the proxy — what Tor needs).

The crate is consumed by inlining (`eth_rpc_module` / `token_list_module` carry
it as `src/proxy.rs`), and its end-to-end refusal is already exercised by those
modules' doc-tests. This doc-test pins down the **invariant at the source**: it
runs the crate's fail-closed test suite in a reproducible Nix toolchain and
shows every case passing.

**What you'll build:** The `logos-net-proxy` crate's fail-closed test suite, run in a reproducible Nix dev shell.

**What you'll learn:**

- Why a single fail-closed client constructor is the wallet's privacy chokepoint
- Which conditions force a refusal (proxy required + unset/blank/garbage/unsupported scheme)
- Which conditions are allowed (no proxy required, or a valid socks5h:// proxy)

## Prerequisites

- **Nix** with flakes enabled. Install from [nixos.org](https://nixos.org/download.html), then enable flakes:

```bash
mkdir -p ~/.config/nix
echo 'experimental-features = nix-command flakes' >> ~/.config/nix/nix.conf
```

- **A Linux or macOS machine** with `python3` available (used to locate the pinned crate source).

---

## Step 1: The invariant

`build_client(cfg)` is the only constructor of a `reqwest::Client` in the
wallet. It refuses — returns `Err` — whenever a proxy is required but cannot
be honored, and otherwise returns a usable client:

| Case | `proxy_required` | `proxy` | Result |
|---|---|---|---|
| `fail_closed_when_required_and_unset`  | `true`  | `None`            | **refuse** |
| `fail_closed_when_required_and_blank`  | `true`  | `""`              | **refuse** |
| `rejects_unsupported_scheme`           | `true`  | `https://…`       | **refuse** |
| `rejects_garbage_proxy`                | `true`  | not a URL         | **refuse** |
| `ok_when_not_required_and_unset`       | `false` | `None`            | allow |
| `ok_with_socks5h_proxy`                | `true`  | `socks5h://…`     | allow |

---

## Step 2: Run the fail-closed suite

The crate's Nix build runs the test suite in its `checkPhase`
(`rustPlatform` sets `doCheck = true`), so building it *is* a reproducible,
green run of the fail-closed invariant — and `-L` streams the `cargo test`
output into the build log.

### 2.1 Build the crate — its checkPhase runs cargo test

```bash
nix build github:logos-co/logos-evm-net-proxy#default -L   # checkPhase = cargo test
```

A successful build is a green run: `checkPhase` executes `cargo test`, so
all six cases above passed — every "proxy required but unhonorable"
configuration refused, and only a missing-and-not-required proxy or a
valid `socks5h://` proxy yielded a client. The `-L` flag streams that
output (`running 6 tests … test result: ok. 6 passed`) into the log
above. That is the guarantee every wallet module inherits by building its
outbound client through this one crate.
