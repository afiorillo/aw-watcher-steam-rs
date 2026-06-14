# Contributing to aw-watcher-steam-rs

Thanks for your interest in contributing!

## A note on how this project is built

**Most of this codebase was written by [Claude](https://www.anthropic.com/claude)
(Anthropic's "Claude Code" agent), directed and reviewed at varying depth by the
maintainer.** This is disclosed openly and on purpose — you should know what
you're reading and building on.

Practically, that means:

- Large parts of the code, tests, and docs are AI-generated.
- Review depth varies. Some code has been carefully read; some has not. Treat the
  project accordingly (see [SECURITY.md](SECURITY.md)).
- The AGPL-3.0 warranty disclaimer (sections 15–16) very much applies: this
  software is provided **without warranty of any kind**.

## AI / LLM-assisted contributions are explicitly welcome

You may open pull requests that were written or assisted by LLMs / AI coding
agents. You do **not** need to hide or apologize for it — this whole project was
built that way.

We only ask that you:

1. **Disclose it.** Add a trailer such as
   `Co-Authored-By: <model name> <noreply@…>` or mention the tools used in the PR
   description. Honesty about provenance matters more than the provenance itself.
2. **Take ownership of what you submit.** By opening a PR you are vouching for the
   change as if you wrote it yourself, regardless of how it was produced. "The AI
   wrote it" is not a defense for a broken or insecure change.
3. **Make it pass the bar below** (build, tests, lint, format).

## No promise of review

This is a hobby / community project. **The maintainer makes no commitment to
review, respond to, triage, merge, or even read any contribution** — AI-assisted
or human — in any particular timeframe, or at all. Pull requests and issues may be
merged, ignored, closed, or left open indefinitely at the maintainer's sole
discretion.

By submitting a contribution you accept this. If you need a guaranteed review or
support, this is not the right project for that expectation.

This applies equally to security reports — see [SECURITY.md](SECURITY.md) for the
(also best-effort) reporting process.

## Licensing of contributions

This project is licensed under **AGPL-3.0-or-later** (see [LICENSE](LICENSE)). By
submitting a contribution you agree that it is licensed under the same terms and
that you have the right to contribute it.

## Quality bar

Even though review is not guaranteed, contributions are expected to clear the same
automated bar the maintainer uses. Before opening a PR, please make sure the
following pass. The repo ships a Nix dev shell (`shell.nix`) that provides the
toolchain plus `openssl`/`pkg-config`:

```sh
nix-shell --run 'cargo build --all-features'
nix-shell --run 'cargo test'
nix-shell --run 'cargo test --no-default-features'   # local-only build
nix-shell --run 'cargo clippy --all-targets'         # no warnings
nix-shell --run 'cargo fmt --check'
```

Guidelines:

- Add or update tests for behavior changes. Keep pure logic unit-testable without
  a live Steam install or aw-server (see the existing tests in `src/steam.rs`,
  `src/report.rs`, `src/config.rs`).
- Keep both feature configurations compiling and warning-free: default
  (`steam-api` on) and `--no-default-features` (local-only).
- Match the surrounding code style; run `cargo fmt`.

## Design constraints to respect

- **Privacy first.** Local detection must read as little as possible. Today it
  refreshes and inspects **only each process's executable path**, never command
  lines, environment, working directories, or window titles, and it discards
  non-matching processes immediately. Do not broaden what is read without a
  discussion and a corresponding update to the privacy disclosure in the README
  and [SECURITY.md](SECURITY.md).
- **Zero-config by default.** The watcher must run with no config file and no
  network. Configuration and the Steam Web API layer are strictly optional.
- **Cross-platform.** Changes should keep Windows, macOS, and Linux (native,
  Flatpak, Snap, Steam Deck) in mind.

## Commit / PR conventions

- Keep commits focused; write descriptive messages.
- Reference any related issue in the PR description.
- If your change affects user-visible behavior, update the README.
