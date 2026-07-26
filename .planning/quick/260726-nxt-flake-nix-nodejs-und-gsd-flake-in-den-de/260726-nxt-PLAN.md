---
phase: quick-260726-nxt
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - flake.nix
  - flake.lock
autonomous: true
requirements: [QUICK-260726-NXT]

must_haves:
  truths:
    - "`nix develop -c node --version` exits 0 and prints a Node version"
    - "`nix develop -c which gsd-tools` resolves to a /nix/store path"
    - "`nix develop -c cargo --version` still exits 0 (Rust toolchain unbroken)"
    - "The flake still evaluates (`nix flake show` succeeds)"
  artifacts:
    - path: "flake.nix"
      provides: "gsd flake input + nodejs and gsd package in devShell buildInputs"
      contains: "gsd.url = \"github:neosam/gsd-flake\";"
    - path: "flake.lock"
      provides: "Pinned revision of the new gsd input"
      contains: "\"gsd\""
  key_links:
    - from: "flake.nix inputs.gsd"
      to: "flake.nix outputs function args"
      via: "outputs = { self, nixpkgs, flake-utils, jj-ws, gsd, rust-overlay }:"
      pattern: "outputs = \\{[^}]*gsd[^}]*\\}"
    - from: "flake.nix outputs arg `gsd`"
      to: "devShells.default.buildInputs"
      via: "gsd.packages.${system}.default entry"
      pattern: "gsd\\.packages\\.\\$\\{system\\}\\.default"
---

<objective>
Make `node` and the `gsd-tools` binary available inside `nix develop` for the haushalt repo.

Purpose: `gsd-core/bin/gsd-tools.cjs` is a Node script. Neither the system PATH nor the current
devShell provides `node`, so every GSD workflow (`/gsd-plan-phase`, `/gsd-execute-phase`, ...)
dies at its init step. Today's only workaround was an ad-hoc `nix shell nixpkgs#nodejs`.

Output: `flake.nix` gains a `gsd` input (`github:neosam/gsd-flake`) plus `nodejs` and
`gsd.packages.${system}.default` in the devShell; `flake.lock` gains the pinned `gsd` node.
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@flake.nix

<reference>
The user pointed at `/home/neosam/programming/projects/shifty-backend/flake.nix` as the
reference implementation. Relevant excerpt (already read during planning — do NOT copy it
wholesale, it contains rejected extras):

```nix
inputs = {
  ...
  openspec.url = "github:Fission-AI/OpenSpec";   # REJECTED for haushalt
  gsd.url = "github:neosam/gsd-flake";           # ADOPT
};

outputs = { self, nixpkgs, rust-overlay, flake-utils, openspec, gsd }:
  ...
  devShells.default = pkgs.mkShell {
    buildInputs = with pkgs; [
      ...
      nodejs                                       # ADOPT
      openspec.packages.${system}.default          # REJECTED
      gsd.packages.${system}.default               # ADOPT
    ];
    CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";  # REJECTED
  };
```
</reference>

<verified_facts>
Already checked during planning — do NOT re-derive:
- `github:neosam/gsd-flake` exposes `packages.x86_64-linux.default` = `gsd-core-1.7.0`,
  providing `bin/gsd-tools`, `bin/gsd-core`, `bin/gsd_run`, `bin/gsd-mcp-server`.
- Those wrappers embed their own nodejs-22.22.2, so `gsd-tools` works even without `nodejs`
  in the shell. `nodejs` is still needed *separately* for `~/.claude/gsd-core/bin/gsd-tools.cjs`.
  Both entries are required; neither replaces the other.
- `flake.lock` will gain a new `gsd` node. That is expected and must be committed with `flake.nix`.
</verified_facts>

<out_of_scope>
These exist in the shifty-backend reference but were DELIBERATELY REJECTED. Do not add them:
- The `openspec` input. Per CLAUDE.md this project migrated from OpenSpec to GSD; `openspec/`
  is historical reference only.
- `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";`. haushalt builds its frontend via
  `trunk`, not a direct `cargo build --target wasm32-unknown-unknown`. Current build works;
  changing the linker is unrelated blast radius.
- Any edit to the `packages` outputs (`backend`, `frontend`, `default`), the `shellHook`,
  or `SQLX_OFFLINE`.
- Any reformat/restructure of the rest of flake.nix.
</out_of_scope>

<environment_notes>
- Nix commands can take several minutes on first evaluation (the `gsd` input must be fetched
  and built). Budget generous timeouts — use `timeout: 600000` on the Bash tool for nix calls.
- `node` is NOT on the default PATH outside the devShell. If needed:
  `/nix/store/x98gls54ki3fmm2pv2cmi6z8mcda6glk-nodejs-24.18.0/bin/node`
- STATE.md line 78 claims `nix develop` needs `NIXPKGS_ALLOW_UNFREE=1 --impure`. That blocker
  is STALE — commit `2e42e4e chore(nix): remove claude-code from devShell` removed the unfree
  package. Plain `nix develop -c ...` is expected to work. If nix nevertheless errors about an
  unfree package, STOP and report it; do NOT silently add `--impure`.
- PRE-EXISTING, unrelated clippy failure at `frontend/src/components/solo_mode_banner.rs:66`
  (`clippy::type_complexity`). Do not fix it, do not let it block this task, do not run clippy.
</environment_notes>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add gsd input and nodejs + gsd package to the devShell</name>

  <read_first>
    - `flake.nix` (whole file — it is only 141 lines)
    - The `<reference>` and `<out_of_scope>` blocks above
  </read_first>

  <files>flake.nix</files>

  <action>
Make exactly three edits to `/home/neosam/programming/projects/haushalt/flake.nix`. Use the
Edit tool for each. Do not touch anything else in the file.

**Edit 1 — add the input (currently lines 4-12).**
Add `gsd.url = "github:neosam/gsd-flake";` directly after the `jj-ws` line. Use the bare `url`
form (mirroring `jj-ws` and the shifty-backend reference), NOT the attrset form. This keeps
all bare-url inputs grouped before the `rust-overlay` attrset. Result:

```nix
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    jj-ws.url = "github:neosam/jj-ws";
    gsd.url = "github:neosam/gsd-flake";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
```

**Edit 2 — thread `gsd` through the outputs signature (currently line 14).**
Insert `gsd` after `jj-ws` so the argument order matches the input declaration order:

```nix
  outputs = { self, nixpkgs, flake-utils, jj-ws, gsd, rust-overlay }:
```

**Edit 3 — extend `devShells.default.buildInputs` (currently lines 104-124).**
Add `nodejs` and `gsd.packages.${system}.default` to the existing `# Development tools`
group, keeping the comment-header grouping style already used in the file.
`jj-ws.packages.${system}.default` is the precedent for referencing a flake-input package
inside the `with pkgs;` block; `nodejs` comes from `pkgs` so it works bare. Result for that
group:

```nix
            # Development tools
            pkg-config
            openssl
            jujutsu
            jj-ws.packages.${system}.default
            # node is required by GSD's gsd-tools.cjs scripts
            nodejs
            gsd.packages.${system}.default
```

Leave the `# Rust toolchain with WASM target`, `# SQLite and sqlx`, and
`# Frontend build tools` groups untouched.

**Then regenerate the lock file.** Run `nix flake lock` (NOT `nix flake update` — that would
bump the existing pins too). This adds only the missing `gsd` node to `flake.lock`.
Budget a 600000 ms timeout; the gsd input must be fetched on first evaluation.
  </action>

  <verify>
    <automated>cd /home/neosam/programming/projects/haushalt &amp;&amp; grep -q 'gsd.url = "github:neosam/gsd-flake";' flake.nix &amp;&amp; grep -q 'outputs = { self, nixpkgs, flake-utils, jj-ws, gsd, rust-overlay }:' flake.nix &amp;&amp; grep -q 'gsd.packages.${system}.default' flake.nix &amp;&amp; grep -q '^ *nodejs$' flake.nix &amp;&amp; grep -q '"gsd"' flake.lock &amp;&amp; ! grep -q 'openspec' flake.nix &amp;&amp; ! grep -q 'CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER' flake.nix &amp;&amp; echo "TASK1 OK"</automated>
  </verify>

  <acceptance_criteria>
    - `flake.nix` contains `gsd.url = "github:neosam/gsd-flake";` in the `inputs` block.
    - The `outputs` signature is `{ self, nixpkgs, flake-utils, jj-ws, gsd, rust-overlay }`.
    - `devShells.default.buildInputs` contains both `nodejs` and `gsd.packages.${system}.default`
      in the `# Development tools` group.
    - `flake.lock` contains a `"gsd"` node, and the pre-existing `jj-ws` / `nixpkgs` /
      `rust-overlay` / `flake-utils` revisions are UNCHANGED (`jj diff flake.lock` shows only
      additions plus the root `inputs` line gaining `"gsd"`).
    - `flake.nix` contains NO `openspec` reference and NO
      `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER`.
    - The verify command above prints `TASK1 OK`.
  </acceptance_criteria>
</task>

<task type="auto">
  <name>Task 2: Verify the devShell and commit with jj</name>

  <read_first>
    - `<environment_notes>` above (stale STATE.md blocker, pre-existing clippy failure, timeouts)
  </read_first>

  <files>flake.nix, flake.lock</files>

  <action>
This is a Nix build-configuration change with no Rust code surface, so these verification
commands ARE the test. Do not invent Rust unit tests for it.

Run each of the following from `/home/neosam/programming/projects/haushalt` with a
600000 ms Bash timeout. Report the actual output of each.

1. Node is available (the primary goal):
   `nix develop -c node --version`
   Must exit 0 and print a version string.

2. The gsd binary resolves:
   `nix develop -c which gsd-tools`
   Must exit 0 and print a `/nix/store/...` path. Then also try
   `nix develop -c gsd-tools --help`. Exit 0 is the goal; if `--help` exits non-zero but
   `which` resolved the binary, that is ACCEPTABLE — record the actual behaviour in the
   summary rather than treating it as a failure.

3. The Rust toolchain is unbroken (no regression):
   `nix develop -c cargo --version`
   Must exit 0.

4. The flake still evaluates:
   Try `nix flake show` first. NOTE: `nix flake check` is the stronger check but may attempt
   to build the frontend/backend packages and take a very long time. If `nix flake check` is
   too slow or times out, `nix flake show` plus a successful `nix develop -c true` is an
   ACCEPTABLE substitute for proving the flake evaluates. If you downgrade, say so
   EXPLICITLY in the summary — do not silently substitute.

5. Sanity-check that no existing tool was displaced:
   `nix develop -c trunk --version` and `nix develop -c jj --version` — both exit 0.

Do NOT run `cargo clippy` — there is a known pre-existing unrelated failure in
`frontend/src/components/solo_mode_banner.rs:66`.

Once all checks pass, commit with jujutsu (this repo uses jj; git is colocated on a detached
HEAD — do NOT use `git commit`):

```
jj commit -m "chore(nix): add nodejs and gsd-flake to the devShell"
```

Both `flake.nix` and `flake.lock` are in the working copy and will be included automatically.
Confirm afterwards with `jj st` (should report no changes) and `jj log -r @- --no-graph -T
'description'`.
  </action>

  <verify>
    <automated>cd /home/neosam/programming/projects/haushalt &amp;&amp; nix develop -c node --version &amp;&amp; nix develop -c which gsd-tools &amp;&amp; nix develop -c cargo --version &amp;&amp; nix develop -c trunk --version &amp;&amp; nix develop -c true &amp;&amp; nix flake show &gt;/dev/null &amp;&amp; echo "TASK2 OK"</automated>
  </verify>

  <acceptance_criteria>
    - `nix develop -c node --version` exits 0 and prints a version.
    - `nix develop -c which gsd-tools` exits 0 and prints a /nix/store path
      (`gsd-tools --help` exit status recorded in the summary either way).
    - `nix develop -c cargo --version` exits 0.
    - `nix develop -c trunk --version` and `nix develop -c jj --version` exit 0.
    - `nix flake show` succeeds (or `nix flake check` succeeds; if downgraded from `check` to
      `show`, the summary states this explicitly).
    - `jj commit -m "chore(nix): add nodejs and gsd-flake to the devShell"` succeeded and
      `jj st` reports a clean working copy.
    - No `git commit` was used.
  </acceptance_criteria>
</task>

</tasks>

<verification>
Single combined gate, runnable from the repo root:

```bash
cd /home/neosam/programming/projects/haushalt
grep -q 'gsd.url = "github:neosam/gsd-flake";' flake.nix \
  && grep -q 'gsd.packages.${system}.default' flake.nix \
  && grep -q '"gsd"' flake.lock \
  && nix develop -c node --version \
  && nix develop -c which gsd-tools \
  && nix develop -c cargo --version \
  && echo "ALL GREEN"
```

Regression guard — these must all be FALSE:
```bash
grep -q 'openspec' flake.nix || echo "ok: no openspec input"
grep -q 'CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER' flake.nix || echo "ok: no wasm linker override"
```
</verification>

<success_criteria>
- `node` is on PATH inside `nix develop`, so `gsd-core/bin/gsd-tools.cjs` can run and GSD
  workflows no longer die at their init step.
- `gsd-tools` (from `github:neosam/gsd-flake`) is on PATH inside `nix develop`.
- Rust toolchain, trunk, sqlx-cli, jujutsu and jj-ws still work in the devShell.
- `flake.nix` diff is limited to: one new input line, one changed `outputs` signature line,
  and three added lines in `buildInputs` (comment + `nodejs` + `gsd.packages.${system}.default`).
- `flake.lock` diff adds only the `gsd` node and the root `inputs` reference to it.
- Change committed with `jj commit`, working copy clean.
</success_criteria>

<notes>
Follow-up for the orchestrator (NOT part of this plan's tasks): `.planning/STATE.md` line 78
records a blocker stating that `nix develop` needs `NIXPKGS_ALLOW_UNFREE=1 --impure` and that
there is "no cargo and no node". That blocker is now doubly stale — `claude-code` was removed
from the devShell in commit `2e42e4e`, and this change adds `node`. Worth clearing during the
STATE.md update step.
</notes>

<output>
After completion, create
`.planning/quick/260726-nxt-flake-nix-nodejs-und-gsd-flake-in-den-de/260726-nxt-SUMMARY.md`
recording: the actual `node --version` output, the `gsd-tools` resolution path and whether
`--help` exited 0, whether `nix flake check` was run or downgraded to `nix flake show`, and
the jj commit id.
</output>
