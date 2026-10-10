# Guardrails: what CI checks, why, and how to get past each check

For the owner (you never need to read code) and for every Claude session. The idea: **the rules live in the repository, not in anyone's memory or prompt.** A failing check is information. Fix the code, or ask for an exception the way this page describes. **Never weaken a check to get a pull request through**: `tools/ci/`, `.github/`, `.claude/`, `clippy.toml`, `rustfmt.toml` and `docs/swarm/` belong to ARCH, and the lane guard rejects a lane's pull request that touches them.

## When the checks run

| When | What runs |
|---|---|
| Every pull request into `integration` or `main` (`pr.yml`) | **guards** (about a minute): checks 1-6 below plus the checks' own tests. **rust**: formatting, clippy, tests of the crates the PR touches, and always the first-light scenario test. |
| Every merge into `integration` or `main` (`integration.yml`) | The whole workspace tested on Linux and Windows, debug and release. The first-light output is kept as a downloadable artifact. |
| Every night (`nightly.yml`) | The 1,000-design fuzz and the validation dashboard. Placeholders until lane VALIDATION writes `w5k validation nightly`. |

The scripts that judge a pull request are taken from the branch it is going into, so a PR cannot loosen its own rules. Text that people control (title, description, branch name) is handed to scripts as data and never pasted into a command.

## The checks

| # | Check | What it protects | To pass | Exception |
|---|---|---|---|---|
| 1 | **Lane guard** `lane_guard.py` | One writer per path: lanes never edit each other's files. | Work on a branch named `lane/<lane>/<topic>`. Change only your lane's paths (`ownership.toml`) plus your status file, theory note, `docs/lanes/<lane>/`, `spikes/<lane>/` and `Cargo.lock`. Never touch a protected path (CI, settings, the rules; see below). Need something from another lane? Write `docs/swarm/requests/<lane>-<topic>.md` and carry on with other work. | A contract change: start the PR title with `CCR:`. It may then edit `crates/w5k_contract/**` and make migration edits anywhere except protected paths; the check prints a loud notice and ARCH reviews every line. A wrong or missing owner, or a protected file that must change: file an interface request; ARCH edits it from its own branch. |
| 2 | **Constants lint** `constants_lint.py` | Every physical number has a source; no magic numbers hide in Rust. | Put the number in RON with provenance `SPEC`, `MEASURED`, `ESTIMATE` or `TUNED`. Free without a comment: `0.0 0.5 1.0 2.0 3.0 4.0 0.25` and tolerances smaller than `0.00001` (such as `1e-9`). Not scanned: tests, `w5k_math`, the contract's test doubles. | A mathematical constant or a tolerance: `// const-ok: <reason>` on the same line or the line above. The reason is required. |
| 3 | **Dependency lint** `deps_lint.py` | No third-party code nobody approved. | In `crates/*/Cargo.toml` write `name.workspace = true`, and the name must already be in the root `[workspace.dependencies]`. | Ask ARCH with a decision card; ARCH adds it to the root `Cargo.toml`. |
| 4 | **Media lint** `media_lint.py` | The repository stays small. | PNG, JPG, WEBP at most 400 KB; GIF 800 KB; anything else 1 MB; 8 MB added per PR. No mp4 mov avi mkv zip 7z tar gz exe dll so dylib pdf. Clips are CI artifacts, not git objects: render them in CI and download them from the run. | ARCH lists one exact path in `tools/ci/large_files.allow`. |
| 5 | **Golden guard** `golden_guard.py` | Goldens (recorded answers under `tests/golden/`, and `*.golden.*` files) are regenerated only deliberately, never to make a failing test pass. | If a golden changes, put a line in the PR description: `Golden-Change: <what changed on purpose and why the new numbers are right>`. | None. No reason, no merge. If a test fails, fix the code. |
| 6 | **Line budget** `line_budget.py` | A crate that keeps growing is a crate nobody can hold in their head. | Stay within `docs/swarm/budgets.toml`: 8,000 lines by default; `w5k_math` 2,500, `w5k_contract` 7,000, `w5k_sim` 5,000, `w5k_tools` 9,500 (a stopgap until ARCH's scenario modules move to their own crate). Blank and comment lines are free. 90% shows as WARN. | Split the crate or delete dead code. Or ask ARCH (decision card) to raise the cap. |
| 7 | **Clippy bans** `clippy.toml` | Determinism: the same inputs give the same bits on Linux and Windows, and nothing depends on the clock. | No std `sin cos tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh exp exp2 exp_m1 ln ln_1p log log2 log10 powf powi hypot cbrt mul_add sin_cos`: call the `libm` function (add `libm.workspace = true`) or write `x * x`. `sqrt abs min max floor ceil round` are fine. No `Instant::now` or `SystemTime::now`. No `HashMap` or `HashSet` (random order): use `BTreeMap`, `BTreeSet` or a `Vec`. | Tooling that never feeds the simulation (reports, charts, timing a CLI run): `#![allow(clippy::disallowed_types)]` or `#![allow(clippy::disallowed_methods)]` at the top of that module, with a comment saying why it is safe. Never in a simulation crate. |
| 8 | **Formatting** `rustfmt.toml` | One style, no noisy diffs. | Run `cargo fmt --all`. | None. |
| 9 | **Tests** | The change works and broke no neighbour. | `cargo test` runs for the crates the PR touches and every crate that depends on them (the whole workspace if a root file, CI file or `content/` changes). The first-light test always runs. | None. A failing test is a bug, not a rule to relax. |

## Lane guard

Decided from `docs/swarm/ownership.toml` **as it is on the branch the pull request goes into**, so a PR cannot edit its own judge.

- A lane branch `lane/<lane>/<topic>` may change its lane's paths and the common ones. Anything else is refused, naming the lane that owns it.
- **Protected paths** (the `[protected]` table: CI, `.claude/`, the ownership file, `CLAUDE.md`, the root `Cargo.toml`, the swarm rules) cannot be changed by any lane branch: not with a `CCR:` title, not even by a `lane/arch/...` branch. Only ARCH's own branches can (the `[unrestricted]` heads: `arch/*`, `integration`, ARCH's session branch). If a lane needs one changed, it files an interface request. No `[protected]` table means no protected paths.
- A `CCR:` title lets a lane PR change the contract paths and make migration edits anywhere except protected paths; ARCH must review every line.
- Any other branch name is refused unless it is an unrestricted head. A branch from a fork is never trusted by name.

## Run them yourself before you push

```
python3 -I tools/ci/lane_guard.py --base origin/integration --head HEAD --branch "$(git branch --show-current)" --title "Your PR title"
python3 -I tools/ci/constants_lint.py          # also: deps_lint.py, line_budget.py
python3 -I tools/ci/media_lint.py --base origin/integration --head HEAD
python3 -I tools/ci/golden_guard.py --base origin/integration --head HEAD --body-file my-pr-description.txt
python3 -I tools/ci/affected.py --base origin/integration --head HEAD     # which crates cargo test will cover
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
```

Each script has `--help`. Exit status 0 means pass, 1 means your PR breaks a rule, 2 means the setup is broken (tell ARCH); `affected.py` only prints a selection. ARCH, after changing a check: `python3 -I -m unittest discover -s tools/ci/tests`.

`package_testdrive.py` is not a check: it builds the owner's test-drive zip (the program, content, the browser page, START.bat and a README) and `integration.yml` runs it on Windows once `tools/viewer/dist/index.html` exists; the zip is kept as the artifact `Warzone5000-TestDrive-windows`.

## What a cloud session may run without asking (`.claude/settings.json`)

Allowed: cargo (build, check, test, clippy, fmt, run, metadata, fetch, tree, clean, and exactly `cargo update -w` for `Cargo.lock`), everyday git (including `reset`, `pull`, `cherry-pick`, `revert`), `python3 -I tools/...`, node, `npm ci`, `npm run`, ffmpeg, `W5K_BLESS=1 cargo test` (re-blessing a golden, which the golden guard then asks you to explain), basic file tools, `gh pr` create/view/edit/list/checks/diff and `gh run` list/view. **Refused outright:** the common spellings of a force push (`--force`, `-f`, `--force-with-lease`, a `+branch` refspec), `git reset --hard`, `git clean`, recursive `rm`, `find -delete`. The permission syntax cannot say "except /tmp", so recursive `rm` is refused everywhere; use `cargo clean` for `target/`. **Not pre-approved**, so an unattended session is refused and should file a request instead: `cargo add`, `cargo install`, plain `cargo update`, `npm install`, `pip`, `curl`, `sudo`, plain `rm`. GitHub tools a lane may use without asking: open and update pull requests, and read pull requests, files, commits, branches, code search, issues, CI runs, check runs and job logs; add a comment. **Lane tool guard** (`.claude/hooks/lane-tool-guard.sh`; a hook rather than a deny rule, because ARCH's own session shares this settings file): on a branch named `lane/<lane>/<topic>` it blocks the tools only ARCH may use. That means the `claude-code-remote` tools that start, interrupt, rename, tag or archive sessions, or message any session other than ARCH's own (a lane may `send_message` to ARCH's session, id in `.claude/hooks/arch-session-id`, by the owner's instruction of 2026-10-10; see RULES.md section 5b), create, change, fire or delete triggers and schedules (`send_later` included), or attach repositories; its read-only tools (`get_session`, `list_sessions`, `list_events`, `read_documentation`, `subscribe_pr_activity`, ...) stay available, because a lane that is stopped by its environment needs `read_documentation` to tell the owner which setting to change. On GitHub the guard blocks merge, auto-merge, `push_files`, `create_or_update_file` and `delete_file`. A blocked lane is told to open a pull request and let ARCH merge, file an interface request, or write a decision card. On any other branch, and on any error, the hook allows the call, so it cannot break ARCH's session. A SessionStart hook (`.claude/hooks/session-start.sh`) runs in cloud sessions only: it prepares `cargo fetch` and the viewer packages, prints a reminder to read `CLAUDE.md` and your lane brief, and never fails the session.

## Getting an exception: the only ways

- Need another lane's file or a change to a contract: an **interface request** (`docs/swarm/requests/<lane>-<topic>.md`), or a `CCR:` pull request for the contract itself.
- Need a new dependency, a bigger crate budget or a bigger file: a **decision card** to ARCH, who edits the root `Cargo.toml`, `budgets.toml` or `large_files.allow`.
- A justified constant: `// const-ok: <reason>`. A justified golden change: `Golden-Change: <reason>`.
- Never: edit a check, a threshold or a workflow; add an `#[allow(clippy::disallowed_...)]` in a simulation crate; name a branch `arch/...` to look trusted. If you believe a check is wrong, write it up as an interface request and move on to other work.

## Limits worth knowing (owner switches in GitHub)

- Branch names are self-declared, so `arch/*` and `integration` are trusted by name only. In GitHub, require pull requests and the checks `guards` and `rust` on `integration` and `main`, restrict who may push `integration`, `main` and `arch/*`, and add a CODEOWNERS entry for `.github/`, `tools/ci/`, `.claude/`, `clippy.toml` and `docs/swarm/`.
- Scheduled workflows run only from the default branch. While `main` stays at its first commit the nightly will not fire: make `integration` the default branch, or start `nightly.yml` by hand.
- The permission lists and the lane tool guard catch the common ways to do something, not every trick (the guard trusts the branch name). They stop accidents and runaway cost.
- These checks see structure, not physics. Whether the numbers are right is the VALIDATION lane's job.
