# PLAN — Parallelize experiment-suite runner internally

Date: 2026-05-14
Goal: Make `experiment-suite run` saturate available cores by running (job, seed) pairs concurrently within a single invocation, instead of serializing the cartesian product through `for (idx, (job_idx, seed)) in suite.job_seed_pairs()` at `sim-cli/src/runner.rs:186`.

Scope:
- Internal parallelism over (job, seed) pairs in `run_suite_with_run_id` and `verify_suite_with_run_id` in `sim-rs/sim-cli/src/runner.rs`.
- New `--parallelism N` CLI flag on `experiment-suite run` and `experiment-suite verify`.
- Synchronization for `manifest.json` writes (only piece of cross-job shared state).
- Failure aggregation across concurrent jobs.
- Tests that pin "parallel run produces same artifacts as sequential run" and resume-under-parallelism behaviour.

Out of scope:
- Anything in `sim-core/` (the simulator). Per-(job, seed) determinism is the simulator's contract and is not negotiable.
- Pricing semantics (Family B chain-derived, controller cadence, multiplier-floor, two-lane variants).
- M2/M3 unit-test goldens and M5 suite-level golden hashes — must be bit-identical pre/post refactor.
- Rayon. Tokio is already `features = ["full"]` (sim-cli/Cargo.toml:39) and `run_job` is already async.
- Parallelism *inside* a single (job, seed) — would break per-run determinism.
- `scripts/run-parallel-suites.sh` rework beyond a documentation tweak.
- Memory profiling or fancy adaptive throttling — conservative default + CLI flag is sufficient.
- Committing changes (per user memory: leave working tree dirty).
- Worktrees (per user memory: work on current branch).

## Binding constraints

- **No commits.** Working-tree changes only.
- **No worktrees.**
- **DO NOT change** the simulation core (`sim-core/`). Per-(job, seed) determinism comes from the simulator; the runner orchestrates concurrent jobs without touching how each one runs.
- **DO NOT change** any pricing-related semantics (Family B chain-derived, controller cadence, multiplier-floor, etc.).
- **Preserve** the M2/M3/M5 goldens — each (job, seed) must produce bit-identical output before/after this refactor. The only change is *when* they run, not *what* they produce.
- **Preserve** resume semantics. Manifest's Completed/Running/Failed state machine, the "Running → Pending on reload" recovery behavior in `Manifest::load_or_init` (runner.rs:81-87), and `verify_suite`'s re-hash check must survive.
- **Preserve** all existing CLI flags (`run-id`, etc.). Add a new `--parallelism N` flag; default = `min(available_parallelism(), 8)`.
- **Preserve** the existing event-stream hash output format. The manifest schema can extend with new fields, but existing field semantics must not change.

## Goal-backward verification

Done means all of the following are true:

- `experiment-suite run --parallelism 8 <suite>` on a 15-job × 3-seed suite finishes in roughly `wall_time(sequential) / min(8, distinct_runnable_jobs)`, subject to memory limits. Some non-linearity is acceptable; the order-of-magnitude improvement is the target.
- `cargo test --workspace` passes.
- `cargo test --release -- --ignored determinism` (the M5 suite-level goldens at `sim-cli/tests/determinism.rs`) passes unchanged. Same SHA256 per (job, seed) pre/post refactor.
- M2/M3 unit-test goldens in `sim-core/src/sim/tests/m2_two_lane.rs` and `m3_actors.rs` pass unchanged — these don't touch the runner so they're a no-op confirmation.
- `experiment-suite run` followed by Ctrl-C followed by `experiment-suite run` (same `--run-id`) correctly resumes: all already-Completed pairs are skipped; previously-Running pairs are reset to Pending by `Manifest::load_or_init` and then re-executed.
- `experiment-suite verify --parallelism 8 <suite>` reports `verify ok` for every Completed (job, seed) and matches `--parallelism 1` output line-for-line modulo ordering of log lines.
- Manifest `jobs` map is in deterministic order (BTreeMap by job_name + BTreeMap by seed string) regardless of completion ordering — this is already true today (runner.rs:71) and must stay true.
- Memory peak on the largest suite (15 jobs × 3 seeds, 100-node topology if present, default `--parallelism 8`) stays well within 32 GB. Spot-check with `/usr/bin/time -v` on one run; document the observed peak in the user-facing handoff but do not gate the plan on a specific number.
- A new test `parallel_run_matches_sequential` in `sim-cli/tests/` proves bit-identical `pricing_event_stream.sha256` for every (job, seed) across `--parallelism 1` and `--parallelism 4` on a small suite.

## Task breakdown

### T1: Add `--parallelism` CLI flag

- File: `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`
- Add `parallelism: Option<usize>` to the `Run` and `Verify` subcommand variants:
  ```rust
  Run {
      suite: PathBuf,
      #[arg(long)]
      run_id: Option<String>,
      /// Max concurrent (job, seed) pairs. Default: min(nproc, 8).
      #[arg(long, short = 'P')]
      parallelism: Option<usize>,
  },
  ```
- Add a helper `resolve_parallelism(opt: Option<usize>) -> usize`:
  - If `opt` is `Some(n)` and `n >= 1`, return `n`.
  - Otherwise return `min(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1), 8)`.
- Reasoning for the cap of 8: each parallel job builds its own simulator state (config, topology, event tracker, mempool, metrics collector). With the largest current topology (~100 nodes) this is on the order of a few hundred MB per job. A cap of 8 keeps peak RSS comfortably under 32 GB on the dev machine while saturating most consumer-grade core counts. Users who know their machine can raise via `--parallelism`. Document this in `CLAUDE.md` (see T5).
- Pass the resolved value into `run_suite_with_run_id` and `verify_suite_with_run_id` (signatures updated in T2/T3).
- The `Status` subcommand does not run jobs; no parallelism flag needed there. It already iterates the manifest once and prints.

### T2: Refactor `run_suite_with_run_id` for parallelism

- File: `sim-rs/sim-cli/src/runner.rs`
- New signature:
  ```rust
  pub fn run_suite_with_run_id(
      suite_path: &Path,
      run_id: Option<&str>,
      parallelism: usize,
  ) -> Result<()>;
  ```
  Keep `run_suite(suite_path)` thin-wrappering to `run_suite_with_run_id(suite_path, None, 1)` for unit tests / one-off callers.
- Replace the current `tokio::runtime::Builder::new_current_thread()` at line 181 with `new_multi_thread()`:
  ```rust
  let runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .worker_threads(parallelism.max(1))
      .build()?;
  ```
  `parallelism` is the *job-level* concurrency cap; tokio worker threads should at least match it so spawned tasks aren't queued behind each other. Each `run_job` is mostly compute-bound through `Simulation::run` + the drain task, so worker_threads = parallelism is appropriate. (Note: `run_job` does its own `tokio::spawn` for the drain task, so 1 spawned job uses 2 tokio tasks. `worker_threads = parallelism` is still fine — tokio multiplexes.)
- Wrap the manifest in an `Arc<tokio::sync::Mutex<Manifest>>` constructed inside the runtime. Manifest writes are not on the hot path (one per job state transition; manifest saves do a JSON serialize + fs write but are well under 1 ms even for the largest suites), so a single mutex is simpler and sufficient than a dedicated writer task.
- Resume-aware dispatch (replaces lines 186-284):
  ```rust
  runtime.block_on(async move {
      let manifest = Arc::new(tokio::sync::Mutex::new(manifest));
      let semaphore = Arc::new(tokio::sync::Semaphore::new(parallelism.max(1)));
      let mut join_set: tokio::task::JoinSet<JobOutcome> = tokio::task::JoinSet::new();

      // Snapshot the pending work BEFORE spawning. The job_seed_pairs
      // iteration order is the suite's natural order and gives
      // deterministic dispatch order; the manifest snapshot is taken
      // once so we don't race between "is Completed" checks and our
      // own writes.
      let pending: Vec<(usize, u64)> = {
          let m = manifest.lock().await;
          suite.job_seed_pairs().into_iter().filter(|(job_idx, seed)| {
              let job = &suite.jobs[*job_idx];
              let entry = m.jobs.get(&job.name).and_then(|s| s.get(&seed.to_string()));
              !matches!(entry, Some(e) if e.status == JobStatus::Completed)
          }).collect()
      };
      let total = pending.len();

      for (i, (job_idx, seed)) in pending.into_iter().enumerate() {
          let permit = semaphore.clone().acquire_owned().await?;
          let suite_arc = Arc::clone(&suite_arc);     // see note below
          let manifest = Arc::clone(&manifest);
          let manifest_path = manifest_path.clone();
          join_set.spawn(async move {
              let _permit = permit; // released on task end
              let job_name = suite_arc.jobs[job_idx].name.clone();
              let seed_key = seed.to_string();

              // Manifest transition: Pending -> Running. Hold the lock
              // for the read-modify-write window only.
              {
                  let mut m = manifest.lock().await;
                  if let Some(jobs) = m.jobs.get_mut(&job_name) {
                      jobs.insert(seed_key.clone(), JobEntry {
                          status: JobStatus::Running,
                          started_at_utc: Some(Utc::now()),
                          completed_at_utc: None, output_path: None, error: None,
                      });
                  }
                  m.save(&manifest_path)?;
              }

              let result = run_job(&suite_arc, job_idx, seed).await;
              let job_dir = suite_arc.output_dir.join(&job_name).join(seed.to_string());
              match &result {
                  Ok(summary) => persist_run_artefacts(&job_dir, summary)?,
                  Err(_) => {}
              }
              // Manifest transition: Running -> Completed | Failed.
              {
                  let mut m = manifest.lock().await;
                  let new_entry = match &result {
                      Ok(_) => JobEntry { status: JobStatus::Completed, .. },
                      Err(e) => JobEntry { status: JobStatus::Failed, error: Some(format!("{e:#}")), .. },
                  };
                  if let Some(jobs) = m.jobs.get_mut(&job_name) {
                      jobs.insert(seed_key, new_entry);
                  }
                  m.save(&manifest_path)?;
                  // metrics_comparison.txt is cheap (reloads completed summaries from disk)
                  // and produces a valid partial comparison on each completion.
                  write_suite_metrics_comparison(&suite_arc, &m)?;
              }
              Ok::<JobOutcome, anyhow::Error>(JobOutcome { job_name, seed, i, total, result })
          });
      }

      // Drain the JoinSet, aggregating failures.
      let mut failures: Vec<anyhow::Error> = Vec::new();
      while let Some(joined) = join_set.join_next().await {
          let outcome = joined??;  // panics in spawned tasks propagate
          match outcome.result {
              Ok(summary) => tracing::info!(
                  "[{}/{}] done: {} seed={} hash={}",
                  outcome.i + 1, outcome.total, outcome.job_name, outcome.seed,
                  &summary.pricing_event_stream_sha256[..16]),
              Err(e) => {
                  tracing::error!("[{}/{}] FAILED: {} seed={}: {e:#}",
                      outcome.i + 1, outcome.total, outcome.job_name, outcome.seed);
                  failures.push(e.context(format!("job {} seed {} failed",
                      outcome.job_name, outcome.seed)));
              }
          }
      }
      if !failures.is_empty() {
          // Aggregate into one error. Don't fail-fast: let other jobs finish
          // so the manifest reflects the real state of every pair.
          let mut combined = anyhow::anyhow!("{} (job, seed) pair(s) failed", failures.len());
          for e in failures { combined = combined.context(format!("{e:#}")); }
          return Err(combined);
      }
      Ok::<(), anyhow::Error>(())
  })?;
  write_suite_metrics_comparison(&suite, &*manifest.lock().await)?;
  ```
  (Sketch above; the final shape may need an `Arc<Suite>` introduced because the `'static` bound on spawned tasks rules out `&Suite`. Either wrap `suite` in `Arc::new(suite)` at the top and use `Arc::clone` per task, or `clone()` the Suite into each task — `Suite` is `Clone` and shallow, only `PathBuf`s and `Vec<Job>`, so cloning per task is acceptable too. Picker's choice during implementation; document which was chosen.)
- Failure aggregation rationale: aggregate-and-continue rather than fail-fast. A failed (job, seed) does not corrupt the manifest (it ends Failed, recoverable on next `run`), and cancelling sibling jobs on first failure wastes the work they've already started. Document the tradeoff in a code comment. (If anyone later needs fail-fast — e.g. CI — they can wrap with `set -e` and the failed exit code already propagates.)
- Manifest mutex contention: jobs hold the lock only for the duration of `m.save(&manifest_path)` plus the `metrics_comparison.txt` write. The latter does `O(completed_jobs)` filesystem reads of `run_summary.json`. For a 45-job suite that's 45 small JSON reads, ~5-20 ms. Worst case: jobs serialize through these write windows but the compute (simulation runs) is fully parallel. Acceptable.
- Signal handling: the existing `ctrlc` crate dependency (Cargo.toml:25) is currently unused in `experiment-suite`. Don't add custom handling — let SIGINT abort the process, and rely on `Manifest::load_or_init`'s "Running → Pending on reload" recovery (runner.rs:81-87) to clean up on the next run. The mutex-held writes mean the manifest on disk is always consistent at the moment of any SIGINT (no partial saves), because `Manifest::save` does a single `std::fs::write` of pretty-JSON. Document this in a comment.

### T3: Refactor `verify_suite_with_run_id` similarly

- File: `sim-rs/sim-cli/src/runner.rs`
- New signature:
  ```rust
  pub fn verify_suite_with_run_id(
      suite_path: &Path,
      run_id: Option<&str>,
      parallelism: usize,
  ) -> Result<()>;
  ```
- Same JoinSet + Semaphore pattern as T2. No manifest writes during verify — read-only walk — so no mutex needed.
- Failure mode: aggregate mismatches (already done; `mismatches += 1` accumulator). Now aggregate across concurrent tasks. After `join_set` drains, bail if `mismatches > 0`.
- Each spawned task returns `Result<VerifyOutcome>` where `VerifyOutcome` carries `(job_name, seed, stored_hash, fresh_hash, matched: bool)`. Driver aggregates.
- The malformed-stored-hash bail (runner.rs:471-478) should fire as it does today — under parallelism this surfaces as one of the JoinSet results being `Err`; aggregate with the others and bail at the end. Same "don't cancel siblings" rationale as T2.
- The existing unit tests `verify_suite_bails_on_empty_stored_hash` and `verify_suite_bails_on_non_hex_stored_hash` (runner.rs:743-762) call `verify_suite(&suite_path)` (the no-arg wrapper). Add a thin wrapper:
  ```rust
  pub fn verify_suite(suite_path: &Path) -> Result<()> {
      verify_suite_with_run_id(suite_path, None, 1)
  }
  ```
  so those tests keep working unchanged. They lay down a single (the_job, seed=1) pair so parallelism is moot regardless.

### T4: Add concurrency tests

- File: `sim-rs/sim-cli/tests/parallel_runner.rs` (new)
- Test `parallel_run_matches_sequential`:
  1. Build a minimal in-tempdir suite: 1 topology, 1 protocol, 1 demand, 2 jobs, 2 seeds → 4 (job, seed) pairs. Reuse the fixture style from `runner.rs:677` (`lay_down_verify_suite_fixture`) but with real YAML files copied/adapted from `parameters/phase-2-sweep/` so `run_job` actually executes. Smallest possible — `default-slots: 100` is fine — to keep the test under ~5 seconds.
  2. Run with `parallelism = 1`, collect each (job, seed) `pricing_event_stream.sha256`.
  3. Wipe `output_dir`. Run with `parallelism = 4`, collect again.
  4. Assert every (job, seed) hash matches across the two runs.
- Test `partial_failure_leaves_recoverable_manifest`:
  1. Same fixture style, but with one of the jobs configured to fail (e.g. pricing YAML pointing at a path that doesn't exist).
  2. Run with `parallelism = 4`. Expect `run_suite_with_run_id` to return `Err`.
  3. Assert: the manifest exists, the failing (job, seed) is `Failed`, the successful ones are `Completed`. No entries are stuck in `Running`.
- Test `resume_under_parallelism_skips_completed`:
  1. Same minimal 4-pair fixture.
  2. Run with `parallelism = 2`. All 4 complete.
  3. Run again with `parallelism = 4`. Assert: no (job, seed) was re-executed (e.g. by checking file mtimes on `run_summary.json` are unchanged; or by adding a counter via a test-only hook — mtime check is simpler).
- Test `run_id_suffix_still_works_under_parallelism`:
  1. Same fixture, two runs with different `--run-id` values, parallelism = 2 on both.
  2. Assert two distinct `output_dir-<run_id>` directories exist with their own manifests.
- Keep these tests OUT of the `#[ignore]`'d determinism suite — they should run on every `cargo test --workspace`. Wall-clock target: under 30 seconds total for the whole new test file in `--release`.

### T5: Update CLAUDE.md

- File: `/home/will/git/arc-tiered-pricing/CLAUDE.md`
- Under "Running the suites", add a paragraph:
  > **Parallelism.** `experiment-suite run` and `experiment-suite verify` run (job, seed) pairs concurrently by default. Cap is `min(nproc, 8)`; override with `--parallelism N` (`-P N`). Each parallel job owns its own simulator state, so peak RSS scales linearly in `N` — with a 100-node topology, 8-way parallelism is comfortably within 32 GB on the dev machine; raise carefully if your topology grows or your RAM is tight.
- Under "Conventions / gotchas", add:
  > **In-suite parallelism preserves per-(job, seed) determinism.** The suite-level golden hashes and the `verify` subcommand both treat each (job, seed) as the determinism unit. Parallelism changes only the wall-clock interleaving of jobs, not their seeds, inputs, or event streams. The manifest's `BTreeMap`-keyed-by-(job_name, seed_string) layout already gives deterministic on-disk order regardless of completion order.

### T6: Update scripts/run-parallel-suites.sh

- File: `sim-rs/scripts/run-parallel-suites.sh` (if it exists at that path — confirm during implementation)
- Recommended action: **leave the script as-is.** It parallelizes *across* suites; each suite invocation now uses internal parallelism. With cross-suite parallelism = K and intra-suite parallelism = P, total tokio worker threads ≈ K × P. On an 8-core box, K=2 + P=4 is a sensible mix; K=4 + P=2 also works. The script's existing fixed-K behaviour is fine because the script-level user knob is already there (`-j` or equivalent).
- Add a one-line note at the top of the script:
  > `# Each `experiment-suite run` now parallelizes (job, seed) pairs internally (default min(nproc, 8)).`
  > `# If you raise the cross-suite -j flag here, consider lowering --parallelism on each suite to avoid CPU oversubscription.`
- That is the entire change. Do not rewrite the script's job model — over-engineering for a marginal use case.

### T7: Final verification

- `cd sim-rs && cargo build --release` — clean build.
- `cd sim-rs && cargo test --workspace` — all tests pass, including the new ones from T4.
- `cd sim-rs && cargo test --release -- --ignored determinism` — M5 suite-level goldens unchanged.
- Manual smoke 1: pick the smallest suite (e.g. `phase-2-eip1559-smoothing.yaml`), `experiment-suite run --parallelism 1` to a fresh `--run-id A`, then `--parallelism 8` to fresh `--run-id B`. Compare every `pricing_event_stream.sha256` under both output dirs — they must match per (job, seed). Use `diff <(find ...-A -name pricing_event_stream.sha256 | sort | xargs cat) <(find ...-B -name ... | sort | xargs cat)`.
- Manual smoke 2: pick a larger suite (`phase-2-priority-only-unreserved.yaml` or `phase-2-two-lane-both-dynamic.yaml`). Time `experiment-suite run --parallelism 1` and `--parallelism 8` against fresh `--run-id`s. Confirm wall-clock improvement is roughly the parallel speedup ceiling (likely 4-8×; sub-linear due to mutex windows and OS scheduling).
- Manual smoke 3: start `experiment-suite run --parallelism 4` on a suite, Ctrl-C after ~5 seconds, re-run same command (same default `--run-id` which means none, so same `output_dir`). Confirm logs show "skip (completed)" for the pairs that finished and re-runs only the rest. Manifest under `output_dir/manifest.json` has no `running` entries after the kill (they're reset by `Manifest::load_or_init` on re-load).

## Task DAG / sequencing

```
T1 (CLI flag) ─┐
               ├─► T2 (run parallel)  ─┐
               └─► T3 (verify parallel)┤
                                       ├─► T4 (tests) ─► T7 (verify)
T5 (CLAUDE.md doc)  ─────────────────► T7
T6 (script doc)  ─────────────────────► T7
```

- T1 must come first (T2/T3 need the new parameter).
- T2 and T3 are file-independent within `runner.rs` but touch the same file, so do T2 first, then T3 (small mechanical follow-up).
- T4 depends on T2 (and T3 if it tests verify, which it doesn't have to).
- T5 and T6 are doc-only, can run in parallel with T4.
- T7 is the gate; everything funnels in.

## Risks & mitigations

- **Risk:** memory explosion at high `--parallelism` on a topology with many nodes. **Mitigation:** conservative default of `min(nproc, 8)`. CLI flag lets users raise if they know their machine. T5 documents the linear-in-N peak-RSS relationship so users understand the knob.
- **Risk:** non-deterministic manifest entry order or per-job artifact layout. **Mitigation:** the manifest already uses `BTreeMap<String, BTreeMap<String, JobEntry>>` (runner.rs:71) which is keyed by (job_name, seed_string), so on-disk order is the natural sort regardless of completion order. Per-job output paths `<output_dir>/<job_name>/<seed>/` are already isolated. No new mechanism needed.
- **Risk:** partial-failure or Ctrl-C leaving the manifest corrupt. **Mitigation:** manifest is single-mutex-protected and `Manifest::save` does one atomic `std::fs::write` of pretty-JSON, so on-disk state is always consistent at SIGINT. `Manifest::load_or_init` resets `Running` → `Pending` on next load (runner.rs:81-87), giving free recovery. Document in T2 code comments.
- **Risk:** the existing tokio runtime is `new_current_thread()` (runner.rs:181, 441). **Mitigation:** T2 and T3 explicitly switch to `new_multi_thread().worker_threads(parallelism.max(1))`. Verified `tokio = { version = "1", features = ["full"] }` in Cargo.toml:39, so `rt-multi-thread` is included.
- **Risk:** rare race between manifest mutex and per-job artifact writes. **Mitigation:** per-job artifact writes (run_summary.json, time_series.csv, diagnostics.log, pricing_event_stream.sha256) land in `<output_dir>/<job_name>/<seed>/`, which is unique per (job, seed). No two parallel jobs ever touch the same file. Only `manifest.json` is cross-job state and it's mutex-protected.
- **Risk:** `metrics_comparison.txt` being rewritten under the mutex on every job completion serializes job ends. **Mitigation:** Acceptable. It's a `O(completed_jobs)` summary-reload + small text write; under 100 ms even for a full suite. If profiling later shows this is a bottleneck, demote to a once-at-suite-end write and remove the per-completion call — but document in T2 that the per-completion call is deliberately kept so long-running suites have inspectable partial comparison output (preserves the behaviour at runner.rs:251).
- **Risk:** `Suite` lifetime in spawned tasks. **Mitigation:** wrap in `Arc<Suite>` at the top of `run_suite_with_run_id`. `Suite` is small (PathBufs + Vec<Job>) so this is cheap. Alternatively `.clone()` into each task — also acceptable.
- **Risk:** existing M5 determinism test at `sim-cli/tests/determinism.rs` invokes the runner. **Mitigation:** that test calls the public API; T2/T3 keep `run_suite(&path)` (no args) as a thin wrapper at `parallelism = 1`. The determinism test is unaffected and continues to assert bit-identical hashes. After T7 manual-smoke confirms parallel mode also produces identical hashes, we know parallelism is safe — but the M5 test does not need to change.

## Out of scope

- Rayon adoption (tokio is sufficient and already a dep).
- Parallelism *within* a single (job, seed) simulation (would break per-run determinism).
- Migrating wrapper scripts beyond a one-line doc note in `run-parallel-suites.sh`.
- Memory profiling or adaptive throttling beyond conservative defaults.
- Cross-architecture CI verification (already explicitly out of scope per CLAUDE.md).
- Committing changes or tagging anything.
