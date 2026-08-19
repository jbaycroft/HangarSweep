# Contributing to HangarSweep

Thanks for your interest in contributing. This document covers everything you need to get started.

---

## Getting started

### Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust (MSVC toolchain) | stable | `rustup install stable` |
| VS Build Tools 2022 | Latest | "Desktop development with C++" workload |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) |
| WebView2 | Any | Pre-installed on Windows 10/11 |

### Clone and install

```powershell
git clone git@github.com:jbaycroft/HangarSweep.git
cd HangarSweep
npm install
```

### EVE credentials for local development

HangarSweep uses EVE SSO. To build a local dev version that authenticates:

**Option A — Use our public app (easiest)**

The `client_id` in source is our public developer application. Set the secret at build time:

```powershell
$env:EVE_CLIENT_SECRET = "your-secret-here"  # ask the maintainer for dev secret
npm run tauri dev
```

**Option B — Register your own EVE developer application (recommended for forks)**

1. Go to [developers.eveonline.com](https://developers.eveonline.com/applications/)
2. Create a new application:
   - Connection: Authentication & API Access
   - Callback: `http://localhost:57423/callback`
   - Scopes: `publicData esi-assets.read_assets.v1 esi-universe.read_structures.v1 esi-ui.open_window.v1 esi-ui.write_waypoint.v1`
3. Update `src-tauri/src/auth.rs` with your `CLIENT_ID`
4. Set `EVE_CLIENT_SECRET` env var with your secret

---

## Development workflow

```powershell
# Hot-reload dev mode (Rust recompiles on change, frontend hot-reloads)
npm run tauri dev

# Type-check frontend only
npx tsc --noEmit

# Run frontend unit tests
npm test

# Run frontend tests in watch mode
npm run test:watch

# Run Rust tests
cd src-tauri
cargo test

# Run Rust linter (must pass with zero warnings)
cargo clippy -- -D warnings

# Check Rust formatting
cargo fmt --check
```

---

## Code style

### Rust
- Follow `rustfmt` defaults — run `cargo fmt` before committing
- Zero `cargo clippy` warnings — we run `clippy -- -D warnings` in CI
- Use `anyhow::Result` for error propagation in async functions
- Prefer explicit error messages in `anyhow!()` calls

### TypeScript / React
- Strict mode (`"strict": true` in tsconfig)
- No `any` types
- Functional components only; no class components
- Keep state in the closest common ancestor

---

## Testing

### Rust tests (in `src-tauri/src/`)
- `db.rs`: integration tests use `sqlite::memory:` with full migration applied
- `auth.rs`: pure unit tests for PKCE generation, URL encoding, JWT parsing
- Run with `cargo test --lib`

### Frontend tests (in `src/__tests__/`)
- Pure utility functions tested with Vitest
- No browser DOM required for current tests (`environment: "node"`)
- Run with `npm test`

### Writing new tests
- Every new DB query should have a test that covers the happy path and at least one exclusion case
- Every new utility function in `types.ts` should have a test in `src/__tests__/`

---

## Submitting a pull request

1. Fork the repo and create a branch: `feature/my-feature` or `fix/the-bug`
2. Make your changes; ensure all tests pass:
   ```powershell
   cargo test --lib
   cargo clippy -- -D warnings
   npm test
   npx tsc --noEmit
   ```
3. Open a PR — fill in the PR template
4. CI will run automatically; address any failures before requesting review

---

## Commit messages

We use conventional commits (loosely):

```
feat: short description of new feature
fix: what was broken and how it's fixed
refactor: code change with no functional difference
docs: documentation only
test: add or update tests
ci: changes to GitHub Actions workflows
chore: dependency updates, config changes
```

---

## Questions

Open a [GitHub Discussion](https://github.com/jbaycroft/HangarSweep/discussions) for anything that's not a bug or feature request.
