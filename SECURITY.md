# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅ Yes     |

---

## Reporting a Vulnerability

**Please do not report security vulnerabilities via public GitHub issues.**

To report a vulnerability, email: **[open an issue marked `security` and we will arrange private disclosure]**

Or open a [GitHub Security Advisory](https://github.com/jbaycroft/HangarSweep/security/advisories/new) (private by default).

We will acknowledge reports within 72 hours and aim to provide a fix or mitigation within 14 days for critical issues.

---

## Threat Model

HangarSweep is a **local native desktop application**. Understanding its threat model helps assess risk correctly.

### What HangarSweep does
- Connects to the official EVE Online ESI API using OAuth 2.0 PKCE
- Stores EVE character access and refresh tokens in a local SQLite database
- Reads asset and market data from ESI; writes nothing back to EVE

### Data stored locally
| Data | Location | Risk if compromised |
|---|---|---|
| Access tokens (short-lived, ~20min) | `%APPDATA%\com.hangarsweep.desktop\hangarsweep.db` | Eve account access for ~20 min |
| Refresh tokens | Same SQLite file | Can generate new access tokens until revoked |
| Asset inventory | Same SQLite file | Read-only EVE data, no real-world value |
| Market prices | Same SQLite file | Public data |

Tokens are stored in plaintext in SQLite. If an attacker can read your `%APPDATA%` folder, they can extract tokens. This is equivalent risk to any browser that stores OAuth tokens.

### EVE client credentials

The `EVE_CLIENT_ID` is a public identifier — sharing it is expected and normal.

The `EVE_CLIENT_SECRET` is compiled into release builds at CI time via a GitHub Actions secret and is **not present in source code**. However, because this is a native desktop app:

- The secret is embedded in the compiled binary
- Anyone with access to the binary can extract it with `strings` or a hex editor
- This is an **accepted and documented limitation** of native OAuth apps (see [RFC 8252 §8.5](https://www.rfc-editor.org/rfc/rfc8252#section-8.5))
- The PKCE flow provides meaningful security against authorization code interception even without a secret
- The secret adds no confidentiality in a distributed native app context

If you believe our EVE developer application credentials are being abused (e.g., someone built a malicious app using our `client_id`), please contact us and we will rotate the credentials.

### What HangarSweep cannot do
- It cannot modify your EVE account, assets, or wallet
- It cannot initiate market orders or contracts
- The `esi-ui.*` scopes only control the in-game UI on your logged-in client (waypoints, open windows)

### Revocation
You can revoke HangarSweep's access at any time at:
[https://community.eveonline.com/support/third-party-applications/](https://community.eveonline.com/support/third-party-applications/)
