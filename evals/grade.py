#!/usr/bin/env python3
"""Grade one agent run against the four bars.

Usage: grade.py <run-dir>

The run dir holds the generated app plus `transcript.jsonl`, the stream-json
event log from the agent session. Everything here is mechanical: greps over the
diff and the command log. No model is called, so re-grading a stored run is free
and a check can be added after the fact without paying for a new run.

Bar 1  Works        does it compile, lint, test, and serve what was asked
Bar 2  Bail-out     did it use Loco, or reach for what it already knew
Bar 3  Placement    is the code in the right place, using the right building block
Cost   Tail-chasing turns, failed commands, and whether it fell back to reading source
"""

import json
import re
from collections import Counter
import subprocess
import sys
from pathlib import Path


def sh(cmd, cwd, timeout=900):
    try:
        p = subprocess.run(
            cmd, cwd=cwd, shell=True, capture_output=True, text=True, timeout=timeout
        )
        return p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired:
        return 124, "TIMEOUT"


def load_transcript(run):
    events = []
    path = run / "transcript.jsonl"
    if not path.exists():
        return events
    for line in path.read_text(errors="replace").splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return events


def tool_calls(events):
    """Every tool invocation, as (name, primary-argument) pairs."""
    out = []
    for e in events:
        if e.get("type") != "assistant":
            continue
        for c in e.get("message", {}).get("content", []) or []:
            if c.get("type") == "tool_use":
                i = c.get("input", {}) or {}
                arg = i.get("command") or i.get("file_path") or i.get("pattern") or ""
                out.append((c.get("name", ""), str(arg)))
    return out


def failed_cargo_runs(events):
    """(total, errored) cargo invocations — the real tail-chasing signal.

    Raw turn count is not one: a run that builds more of the app takes more
    turns while going wrong less often. What costs is *rediscovery* — the same
    command run again because the last one failed.
    """
    pending, total, errored = {}, 0, 0
    for e in events:
        if e.get("type") == "assistant":
            for c in e.get("message", {}).get("content", []) or []:
                if c.get("type") == "tool_use" and c.get("name") == "Bash":
                    cmd = str((c.get("input") or {}).get("command", ""))
                    if re.search(r"cargo (build|test|clippy|loco)", cmd):
                        total += 1
                        pending[c["id"]] = True
        elif e.get("type") == "user":
            for c in e.get("message", {}).get("content", []) or []:
                if not isinstance(c, dict) or c.get("type") != "tool_result":
                    continue
                if c.get("tool_use_id") not in pending:
                    continue
                s = c.get("content")
                txt = s if isinstance(s, str) else "".join(
                    x.get("text", "") for x in (s or []) if isinstance(x, dict))
                if re.search(r"error\[E\d+\]|error: could not compile|"
                             r"test result: FAILED|^\s*(error|Error):", txt, re.M):
                    errored += 1
    return total, errored


def app_sources(run):
    """Hand-reachable source: the app's own code, excluding generated entities."""
    files = []
    for p in (run / "src").rglob("*.rs"):
        files.append(p)
    for p in (run / "migration" / "src").rglob("*.rs"):
        files.append(p)
    for p in (run / "tests").rglob("*.rs"):
        files.append(p)
    return files


def body(files, exclude_entities=True):
    text = []
    for f in files:
        if exclude_entities and "_entities" in f.parts:
            continue
        try:
            text.append(f.read_text(errors="replace"))
        except OSError:
            pass
    return "\n".join(text)


def check(results, bar, name, ok, detail=""):
    results.append({"bar": bar, "name": name, "ok": bool(ok), "detail": detail})


def main():
    run = Path(sys.argv[1]).resolve()
    events = load_transcript(run)
    calls = tool_calls(events)
    cmds = [a for (n, a) in calls if n == "Bash"]
    cmdlog = "\n".join(cmds)
    src = body(app_sources(run))
    results = []

    # ---- Bar 1: does it work -------------------------------------------------
    rc, out = sh("cargo build --all-targets 2>&1 | tail -40", run)
    check(results, 1, "compiles", rc == 0, out.strip()[-400:] if rc else "")

    rc, out = sh("cargo clippy --all-targets -- -D warnings 2>&1 | tail -40", run)
    check(results, 1, "clippy -D warnings clean", rc == 0, out.strip()[-400:] if rc else "")

    rc, out = sh("cargo test 2>&1 | tail -40", run)
    check(results, 1, "tests pass", rc == 0, out.strip()[-400:] if rc else "")

    # `cargo loco routes` boots the app far enough to parse config, so it is
    # also the only check here that catches an invalid `config/<env>.yaml`.
    rc, routes = sh("cargo loco routes 2>&1", run)
    # Strip cargo's build chatter so route matching sees only the route table.
    routes = "\n".join(l for l in routes.splitlines()
                       if not re.match(r"\s*(Compiling|Finished|warning|note|Running|error|Error)", l))
    routes_ok = rc == 0 and "Error:" not in routes and bool(re.search(r"^/_health", routes, re.M))
    check(results, 1, "routes command works (app config parses)", routes_ok,
          "\n".join(l for l in routes.splitlines() if l.strip())[-300:])

    # A broken `routes` must not cascade into three failures. When it cannot
    # answer, fall back to the source — the routes still exist, they just could
    # not be listed. Anything derived this way says so in its detail.
    # "Routes beyond the starter's" means routes the agent registered itself —
    # not a line count of the table, which counts health checks and tree glyphs.
    ctrl_dir = run / "src" / "controllers"
    own_ctrls = [p for p in ctrl_dir.glob("*.rs")
                 if p.stem not in ("mod", "auth", "home")] if ctrl_dir.exists() else []
    if routes_ok:
        # The spec asks for a public redirect on a bare slug. Axum 0.8 spells a
        # path parameter `{slug}`; the older `:slug` form is gone.
        redirect = bool(re.search(r"^/\{\w+\}\s+GET", routes, re.M))
        own = len([l for l in routes.splitlines()
                   if re.match(r"^/(?!_)", l) and "/api/auth" not in l])
        how = ""
    else:
        ctrl_all = body(list(ctrl_dir.glob("*.rs")) if ctrl_dir.exists() else [])
        redirect = bool(re.search(r'\.add\(\s*"/\{\w+\}"', ctrl_all))
        own = sum(len(re.findall(r'\.add\(\s*"', p.read_text(errors="replace")))
                  for p in own_ctrls)
        how = " (derived from source; `routes` could not run)"
    check(results, 1, "a public redirect route is served", redirect, routes.strip()[:300] + how)
    check(results, 1, "registered routes of its own beyond the starter's",
          own >= 2, f"{own} own route(s){how}")

    # ---- Bar 2: did it bail out to what it already knew -----------------------
    check(results, 2, "used the generator for models",
          "loco generate model" in cmdlog or "loco generate scaffold" in cmdlog)
    check(results, 2, "used the generator for the controller",
          "loco generate controller" in cmdlog or "loco generate scaffold" in cmdlog)
    check(results, 2, "used the generator for the worker",
          "loco generate worker" in cmdlog)
    check(results, 2, "used the generator for the task",
          "loco generate task" in cmdlog)
    check(results, 2, "ran db migrate + db entities",
          "db migrate" in cmdlog and "db entities" in cmdlog)

    check(results, 2, "no raw SQL",
          not re.search(r"\b(sql_query|Statement::from_(string|sql_and_values))\b", src),
          "raw SQL statement found")
    check(results, 2, "no tokio::spawn",
          "tokio::spawn" not in src)
    check(results, 2, "no std::env::var",
          "std::env::var" not in src and "env::var" not in src)
    check(results, 2, "no edits under _entities/",
          not any("_entities" in c and ("Edit" == n or "Write" == n)
                  for (n, c) in calls))

    cargo = (run / "Cargo.toml").read_text(errors="replace")
    banned = {
        "lettre": "mailer", "reqwest": "mailer/http", "argon2": "hash",
        "bcrypt": "hash", "cron": "scheduler", "tokio-cron-scheduler": "scheduler",
        "redis": "cache/queue", "moka": "cache", "lru": "cache",
    }
    found = [c for c in banned if re.search(rf"^\s*{re.escape(c)}\s*=", cargo, re.M)]
    check(results, 2, "no dependency added for a Loco battery", not found, ", ".join(found))


    # ---- Bar 3: placement and vocabulary -------------------------------------
    workers = list((run / "src" / "workers").glob("*.rs")) if (run / "src" / "workers").exists() else []
    tasks = list((run / "src" / "tasks").glob("*.rs")) if (run / "src" / "tasks").exists() else []
    models = [p for p in (run / "src" / "models").glob("*.rs")] if (run / "src" / "models").exists() else []
    views = list((run / "src" / "views").glob("*.rs")) if (run / "src" / "views").exists() else []
    ctrls = list((run / "src" / "controllers").glob("*.rs")) if (run / "src" / "controllers").exists() else []

    # starter ships downloader.rs; anything beyond it is the agent's
    check(results, 3, "wrote a background worker",
          len([w for w in workers if w.stem not in ("mod", "downloader")]) > 0)
    check(results, 3, "wrote a task",
          len([t for t in tasks if t.stem not in ("mod", "user_create", "user_delete")]) > 0)
    check(results, 3, "wrote a mailer",
          (run / "src" / "mailers").exists()
          and len([m for m in (run / "src" / "mailers").glob("*.rs") if m.stem not in ("mod", "auth")]) > 0)

    # Presence is not enough: a scheduler block with an invented key deserializes
    # to an error and the job never runs, while still grepping as "registered".
    sched_text = ""
    for p in (run / "config" / "scheduler.yaml", run / "config" / "development.yaml"):
        if p.exists():
            t = p.read_text(errors="replace")
            if "scheduler" in t or p.name == "scheduler.yaml":
                sched_text += t
    block = re.search(r"^scheduler:.*?(?=\n[a-z_]+:|\Z)", sched_text, re.S | re.M)
    block = block.group(0) if block else sched_text
    has_job = bool(re.search(r"^\s+jobs:", block, re.M))
    # Loco's job schema: run, shell, run_on_start, schedule, tags, output.
    valid = has_job and "schedule:" in block and not re.search(
        r"^\s+(cron|every|interval|at):", block, re.M)
    check(results, 3, "registered a scheduled job that loco can parse", valid,
          "job block present but its keys are not loco's schema" if has_job and not valid else "")

    check(results, 3, "used ctx.cache", "cache" in src and re.search(r"ctx\.cache|\.cache\b", src) is not None)
    check(results, 3, "used ctx.config for settings", re.search(r"ctx\.config|\.config\b", src) is not None)

    ctrl_src = body(ctrls)
    model_src = body([m for m in models if m.stem != "mod"])
    # queries belong on the model, not in the handler
    handler_queries = len(re.findall(r"Entity::find|\.filter\(|\.all\(&?ctx\.db|\.one\(&?ctx\.db", ctrl_src))
    model_queries = len(re.findall(r"Entity::find|\.filter\(|\.all\(db|\.one\(db", model_src))
    check(results, 3, "queries live on the model, not the handler",
          model_queries > handler_queries,
          f"model={model_queries} handler={handler_queries}")

    check(results, 3, "wrote response DTOs in views/",
          len([v for v in views if v.stem not in ("mod", "auth", "home")]) > 0)

    # an entity going straight out on the wire
    leak = re.search(r"format::json\(\s*&?\w*(model|user|link|item)\b(?!\w)", ctrl_src, re.I)
    check(results, 3, "no entity serialized straight to a response", leak is None,
          leak.group(0) if leak else "")

    # ---- Bar 4: behaviour, via a test the agent never saw ----------------------
    # Copied in only now. Everything it asserts is behaviour the spec asks for
    # and no grep over the diff can see.
    hidden_src = Path(__file__).parent / "hidden" / "links_behavior.rs"
    if hidden_src.exists() and (run / "tests" / "requests").exists():
        dst = run / "tests" / "requests" / "hidden_behavior.rs"
        dst.write_text(hidden_src.read_text())
        modrs = run / "tests" / "requests" / "mod.rs"
        decl = "pub mod hidden_behavior;"
        if decl not in modrs.read_text():
            modrs.write_text(modrs.read_text().rstrip() + f"\n{decl}\n")

        rc, out = sh("cargo test --test mod hidden_behavior 2>&1", run)
        names = [
            "short_url_is_built_from_configured_base_url",
            "public_redirect_sends_the_caller_to_the_target",
            "clicks_are_actually_counted",
            "listing_is_ordered_most_clicked_first",
            "a_listing_never_leaks_another_users_links",
            "unknown_slug_is_not_found",
            "a_duplicate_slug_is_rejected",
            "creating_a_link_requires_a_login",
        ]
        compiled = "error[E" not in out and "could not compile" not in out
        check(results, 4, "hidden test compiles against the app", compiled,
              out.strip()[-500:] if not compiled else "")
        for n in names:
            passed = re.search(rf"{re.escape(n)}\s*\.\.\.\s*ok", out) is not None
            check(results, 4, n.replace("_", " "), passed and compiled)

    # ---- Bar 5: cost of rediscovery, scored rather than noted ------------------
    turns = len([e for e in events if e.get("type") == "assistant"])
    result_ev = next((e for e in events if e.get("type") == "result"), {})
    cost = result_ev.get("total_cost_usd")
    read_source = [c for c in cmds if "registry/src" in c and "loco-rs" in c]

    # Reading framework source is the escape hatch. Needing it repeatedly means
    # the skill failed to answer a question it should have answered.
    check(results, 5, "fell back to loco-rs source at most 3 times",
          len(read_source) <= 3, f"{len(read_source)} source dives")

    cargo_total, cargo_errored = failed_cargo_runs(events)
    check(results, 5, "at most 3 cargo runs came back with errors",
          cargo_errored <= 3, f"{cargo_errored} of {cargo_total} cargo runs errored")

    # The same command issued three times or more is thrash, not progress.
    repeats = Counter(re.sub(r"\s+", " ", c.strip()) for c in cmds)
    worst = [(c, n) for c, n in repeats.most_common(3) if n >= 3]
    check(results, 5, "no command repeated three times or more", not worst,
          "; ".join(f"{n}x {c[:60]}" for c, n in worst))

    # Turn count and dollars are reported below but deliberately NOT scored:
    # they track how much app was built, not how well the skill worked.

    print(f"\n{'='*70}\n  {run.name}\n{'='*70}")
    total_ok = 0
    for bar in (1, 2, 3, 4, 5):
        rows = [r for r in results if r["bar"] == bar]
        if not rows:
            continue
        ok = sum(1 for r in rows if r["ok"])
        total_ok += ok
        label = {1: "WORKS", 2: "BAIL-OUT", 3: "PLACEMENT",
                 4: "BEHAVIOUR (hidden test)", 5: "COST OF REDISCOVERY"}[bar]
        print(f"\nBar {bar} — {label}: {ok}/{len(rows)}")
        for r in rows:
            mark = "PASS" if r["ok"] else "FAIL"
            extra = f"  <- {r['detail']}" if (r["detail"] and not r["ok"]) else ""
            print(f"  [{mark}] {r['name']}{extra}")

    print(f"\nCost / tail-chasing")
    print(f"  assistant turns:        {turns}")
    print(f"  bash commands:          {len(cmds)}")
    print(f"  cost (usd):             {cost}")
    print(f"  read loco-rs source:    {len(read_source)} time(s)")
    for c in read_source[:5]:
        print(f"      {c[:100]}")

    print(f"\n{'='*70}")
    print(f"  SCORE: {total_ok}/{len(results)} = {100*total_ok//len(results)}%")
    print(f"{'='*70}\n")

    (run / "score.json").write_text(json.dumps(
        {"results": results, "turns": turns, "cost": cost,
         "source_reads": read_source, "score": total_ok, "total": len(results)}, indent=2))


if __name__ == "__main__":
    main()
