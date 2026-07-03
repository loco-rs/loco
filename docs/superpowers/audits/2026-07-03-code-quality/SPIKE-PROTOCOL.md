# Iteration 2 — Spike & Validation Protocol (binding)

You validate Iteration-1 library hypotheses with REAL evidence, not assertion. The user's
rule: "when u think a library might fit — ensure it does and don't assume, perform spikes."

## Hard rules
- **Build a real throwaway cargo crate** for each library hypothesis under
  `/private/tmp/claude-501/-Users-jondot-projects-loco/cc99afe6-72c6-436d-babe-25c5f624e994/scratchpad/spikes/<slug>/`.
  Do NOT touch the loco repo. Do NOT add deps to the loco workspace.
- **Actually compile it** (`cargo build` / `cargo test`). Paste the real command + result.
  A hypothesis is not validated until code compiles and demonstrates the drop-in.
- **Verify the library is current & the API is real**: check the latest version on crates.io
  (use context7 `resolve-library-id`+`query-docs`, or WebFetch docs.rs / crates.io). State
  the exact version you tested and the exact API you used. No hallucinated APIs.
- **Read the incumbent Loco code** you propose to replace (cite file:line) so the comparison
  is real — count the LOC the swap removes vs the LOC/deps it adds.
- **Verdict per hypothesis, one of:**
  - `PROVEN-FIT` — compiled a demo that replaces the incumbent AS SIMPLY or simpler, with
    equal/better behavior; net LOC/dep win. Recommend.
  - `PARTIAL` — works for the common case but loses a real Loco semantic (name the lost
    behavior with a cite). Recommend only with the caveat spelled out.
  - `DOESN'T-FIT` — swap adds code/deps/complexity or breaks a required semantic. Reject,
    with the concrete reason (the incumbent stays; this VALIDATES its KPI7 score).
- Be adversarial toward your own hypothesis. A rejected library is a SUCCESS of the audit
  (it confirms Loco's lean choice), not a failure. Do not force a fit.

## Output
Write full spike report (with pasted compile output) to the file path in your prompt under
`docs/superpowers/audits/2026-07-03-code-quality/spikes/`. Return to orchestrator ONLY:
per-hypothesis verdict line (`PROVEN-FIT|PARTIAL|DOESN'T-FIT — <lib>@<ver> — <one-line reason>
— incumbent @cite, net LOC ±N`).
