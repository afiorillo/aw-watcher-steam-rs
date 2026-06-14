# Security Policy

## Heads-up: AI-generated code

Much of this project was written by an AI coding agent and has been reviewed to
varying (sometimes shallow) depth — see [CONTRIBUTING.md](CONTRIBUTING.md) for the
full disclosure. Audit the code yourself before relying on it in a sensitive or
high-trust environment. As per the AGPL-3.0 (sections 15–16), the software comes
with **no warranty**.

## Supported versions

This is a pre-1.0 hobby project. Only the latest commit on the default branch is
"supported" in any sense; there are no backports or maintenance branches.

| Version            | Supported |
| ------------------ | --------- |
| latest `main`/tip  | ✅        |
| anything older     | ❌        |

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Use **GitHub's private vulnerability reporting** for this repository:
*Security → Advisories → "Report a vulnerability"*. This keeps the report private
until a fix is available.

When reporting, please include:

- A description of the issue and its impact.
- Steps to reproduce (a minimal proof of concept if possible).
- Affected platform(s) and version/commit.

### What to expect

Handling of security reports is **best-effort, with no guaranteed response time —
or response at all**, consistent with the no-promise-of-review policy in
[CONTRIBUTING.md](CONTRIBUTING.md). If you need guaranteed turnaround, please do
not rely on this project. You are welcome to publicly disclose after a reasonable,
good-faith waiting period if you receive no response.

## Threat model & scope

What this watcher actually does, so you can reason about risk:

- **Process inspection (local detection).** It reads the system process list but
  inspects **only each process's executable path**, solely to test whether it
  lives inside a known Steam library folder. It does **not** read command-line
  arguments, environment variables, working directories, or window titles.
  Non-matching processes are discarded immediately and are never stored, logged,
  or transmitted. Only the matched game's name/app-id is emitted.
- **Where data goes.** Detected activity is sent **only to your local
  ActivityWatch server** (default `http://localhost:5600`). Nothing else leaves
  your machine in the default, no-API-key mode.
- **Steam Web API key (optional).** If you enable the Steam Web API layer, your
  API key and SteamID are stored **in plaintext** in the config file. Protect that
  file with appropriate filesystem permissions. The key is sent only to Valve's
  official API endpoints over HTTPS. Treat the key as a secret and revoke it at
  <https://steamcommunity.com/dev/apikey> if exposed.

### In scope

- Vulnerabilities in this project's own code (e.g. unsafe handling of file/config
  input, path-matching flaws, leaking more data than documented above).
- The watcher sending data anywhere other than the configured local server / the
  official Steam API.

### Out of scope

- Vulnerabilities in upstream dependencies (report those upstream; we will bump
  versions as practical).
- Vulnerabilities in ActivityWatch itself or in Steam.
- Issues that require an already-compromised local account/machine.
