<div align="center">
  <h1>tokenleft</h1>

  <p><strong>Show remaining Claude, Codex and Cursor usage</strong></p>

  <p>
    <img alt="License" src="https://img.shields.io/badge/license-MIT-green">
    <img alt="Rust" src="https://img.shields.io/badge/rust-1.85%2B-orange">
    <img alt="Edition" src="https://img.shields.io/badge/edition-2024-blue">
    <a href="https://crates.io/crates/tokenleft"><img alt="crates.io" src="https://img.shields.io/crates/v/tokenleft.svg"></a>
  </p>

  <p>
    <a href="#install">Install</a>
    &nbsp;·&nbsp;
    <a href="#quickstart">Quickstart</a>
  </p>
</div>

---

## Install

Requires [Rust](https://rustup.rs) **1.85+** and `~/.cargo/bin` on your `PATH`.

```bash
cargo install tokenleft
```

Verify:

```bash
tokenleft --help
```

<details>
<summary><strong>Build from source</strong> — for development or unreleased changes</summary>

```bash
git clone https://github.com/cesarferreira/tokenleft.git
cd tokenleft
cargo install --path . --locked
# or
make install-release
```

Debug install (faster compile, larger binary):

```bash
make install
```

Run without installing:

```bash
make build-release
./target/release/tokenleft
```

</details>

<a id="quickstart"></a>
## Quickstart

```bash
tokenleft                    # one-shot bars for Claude, Codex, Cursor
tokenleft --watch            # refresh every 120s (or --watch 30)
tokenleft --refresh          # skip the cache
tokenleft --ttl 60           # cache lifetime in seconds (default 300, 0 = off)
tokenleft --only claude,cursor
tokenleft --format summary   # 3 lines: bar, money used, percent used
tokenleft --format bars      # one compact bar per provider
tokenleft --format rings     # one line of ◔ ◑ ◕ circles
tokenleft --format plain     # uncoloured one-liner for status bars
tokenleft --format json      # machine-readable (--json also works)
```

Results are cached for 5 minutes in `~/Library/Caches/tokenleft/cache.json`, and the last good data is shown if a refresh fails. macOS only. It reads local credentials and calls unofficial endpoints, so it can break without notice:

| Agent | Credentials | Source |
| --- | --- | --- |
| Claude | Keychain item `Claude Code-credentials` (`/usr/bin/security`) | `api.anthropic.com/api/oauth/usage` |
| Cursor | Session JWT in Cursor's `state.vscdb` (`/usr/bin/sqlite3`) | `cursor.com/api/usage-summary` (stale if Cursor.app hasn't run recently) |
| Codex | Managed by `codex app-server` | JSON-RPC `account/rateLimits/read` |

### Examples

**Default** — every window, with spend and reset times:

```text
Agent usage

Claude  Max  · updated just now
  Monthly spend    ━━━━━━━━━━──────────────  $84 / $200  ( 42% used)

Codex  Pro  · updated just now
  Monthly credits  ━━━━━━━━━━━━━━━━────────  3.4k / 5.0k credits  ( 68% used)  resets in 9d 2h

Cursor  Pro  · updated just now
  Your spend       ━━──────────────────────  $1.80 / $20  (  9% used)  resets in 9d 2h
```

**`--format summary`** — three lines: bar, money or credits used, percent used:

```text
Claude ━━━━━━━━────────────  $84 / $200            42%
Codex  ━━━━━━━━━━━━━━──────  3.4k / 5.0k credits   68%
Cursor ━━──────────────────  $1.80 / $20            9%
```

**`--format bars`** — one compact bar per provider:

```text
Claude ▰▰▰▰▱▱▱▱▱▱  42%
Codex  ▰▰▰▰▰▰▰▱▱▱  68%
Cursor ▰▱▱▱▱▱▱▱▱▱   9%
```

**`--format rings`** — a single line, handy for a narrow pane:

```text
Claude ◑ 42%   Codex ◕ 68%   Cursor ○ 9%
```

**`--format plain`** — no colour, for status bars and scripts:

```text
Claude 42% · Codex 68% · Cursor 9%
```

**`--format json`** — for scripts (one provider shown):

```json
[
  {
    "name": "Codex",
    "plan": "Pro",
    "rows": [
      {
        "label": "Monthly credits",
        "used_pct": 68.0,
        "detail": "3.4k / 5.0k credits",
        "resets_at": 1800000000
      }
    ],
    "error": null,
    "fetched_at": 1799000000
  }
]
```

Bars are blue below 70%, yellow from 70% and red from 90%. The numbers above are illustrative.

Use it from a script:

```bash
tokenleft --only cursor --format plain          # Cursor 9%
tokenleft --json | jq '.[] | select(any(.rows[]; .used_pct > 80)) | .name'
tokenleft --watch 300 --format summary          # live, refresh every 5 minutes
```

### Rex

Add to `~/.config/rex/init.lua`. The shortcut opens a small usage popup centered in the window, and pressing it again closes it:

```lua
rex.action{
  name  = "tokenleft",
  title = "Agent usage",
  run   = function(ctx)
    for _, b in ipairs(rex.session.list_blocks{}.blocks or {}) do
      if b.label == "tokenleft" then
        rex.exec({ "rex", "block", "close", "-s", tostring(rex.session_id), b.block_id })
        return
      end
    end
    rex.session.new_layer{
      bounds = { 0.3, 0.45, 0.4, 0.1 },
      layout    = rex.layout.block{
        flavor  = "com.superlogical.terminal.shell",
        label   = "tokenleft",
        options = { command = { "tokenleft", "--watch", "--format", "summary" }, exit = { on_completion = false } },
      },
    }
  end,
}
rex.bind("cmd+shift+u", "tokenleft")
```

## License

MIT
