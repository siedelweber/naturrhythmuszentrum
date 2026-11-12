SPDX-License-Identifier: CC0-1.0

Prepared sigmoid and tanh process evidence

Evidence only, not offered for merge. These observations are from commit
da8183bcc35f2551697f318da5768e08ef43dce0, release/default features,
rustc 249c1e4e31111b54ad2483cfd4cea06c7020c653 (nightly-2026-11-06),
x86_64-unknown-linux-gnu, on 2026-11-12. No process is rerun for this set.
Data and documentation are CC0-1.0; check.py is AGPL-3.0-only.

File map
  256-disabled, 256-enabled, 4096-disabled, 4096-enabled: each folder
  holds six process stdout records (0.jsonl through 5.jsonl), their stderr,
  processes.json and state.json. Ordinal 0 is the discarded warm-up and
  retains its provenance record. Ordinals 1 through 5 are fresh processes.
  The manifests retain PID, warm-up PID, date/build, exit, start/end,
  wall, user/system CPU, binary digest, argv, command and named bindings.
  state.json links the group's successful admission and its before/after
  states, including the post-group idle fractions.
  groups.json: four complete group records and their machine states.
  admissions/0.json: baseline, with no preceding interval and no idle
  fraction invented. admissions/1.json through 7.json: every decision,
  including three refusals, in order. decisions.jsonl contains the same
  records for checking. Each records all core pairs, idle percentages,
  policy and frequency, pressure, load, account-work flag/count, choice
  and refusal terms. The state pressure strings retain avg10, avg60,
  avg300 and cumulative totals; the bound uses avg10 only.
  watch.json: the observation window and completion status.
  accounting/0.txt and 1.txt: the tests' own accounting and result lines.
  accounting/commands.json: runnable cargo arguments, one-job setting,
  toolchain and original launcher wall/CPU/exit receipts. Launcher CPU
  is not the inner test CPU; figures in the test output are logical counts.
  machine.json: processor, pair, policy, boost and refused instruction
  probe, including its diagnostic. Instruction counts are unavailable,
  never zero.
  original-digests.json: anonymous digests and sizes for the retained
  unreduced files. Their private-name mapping is held by the measuring
  party; the reduction can be shown to the reviewers on request.
  manifest.json: every other published file's digest, size and source
  digest(s). It does not claim a self-hash; its own bytes are identified
  by the evidence commit and can be hashed by the row publisher.
  check.py: the artifact/row join, using Python 3.11 or newer, no packages.

Commands and bindings
Build the example once at the named commit and toolchain:
  cargo build --locked --release -p nrz-matrix --example prepared_sigmoid_tanh_measurements
The binary digest is f5e9ebf1f61ecfd496c71240cafcd4a7d164edfc34535346d3c1fcd38ebaa942.
Each process manifest gives the command, its arguments and all NRZ_*
bindings. Run its command in Bash with those bindings exported, with
the working directory at the build's target directory; NRZ_BINARY is
release/examples/prepared_sigmoid_tanh_measurements. The command uses
taskset to pin logical CPU 14. NRZ_COMMAND is the command string itself.
These are runnable spellings of the observed invocations, not a request
to rerun a measurement. A new run needs its own warm-up PID and identity.
Accounting uses the manifest's plain cargo commands at the checkout
root, CARGO_BUILD_JOBS=1 and the named toolchain.

From a checkout containing the rows, run the join with this set available:
  python3 artifacts/prepared-sigmoid-tanh-process-rows/check.py data/inputs/engineering.toml
For regenerated public rows, add --evidence-commit followed by the full
public evidence commit. The same module's published_records function
supplies the exact sources, including process, group-manifest and
admission URLs and digests; no estimate is changed by that operation.
The checker verifies file digests, all admission decisions, process and
warm-up identities, all 424 values and their process variation. It refuses
stale citations and accounting commands separately from semantic errors.
The next row publication cites a process record, its processes.json and
its successful admissions/N.json, each by path and SHA-256 at the public
evidence commit. Accounting cites accounting/0.txt and commands.json.

Admission bound, adopted before these readings
Both siblings must be at least 99 percent idle during the preceding
thirty seconds; I/O wait is not idle. CPU pressure some avg10 is at most
5 percent; memory some avg10 at most 0.1 percent and full avg10 zero.
Frequency policy (driver, governor, energy preference, minimum, maximum
and boost) is present and unchanged across that interval; current
frequency is recorded. No work of the measuring account runs besides
monitor, transport and housekeeping. The reduced flag/count records
the original monitor's classification, not an independent public audit
of the omitted command lines. Counts include non-housekeeping process
observations plus service observations, not deduplicated workloads.
One-minute load is recorded, not an admission bound. Prefer the previous
pair; a new eligible pair may be selected after five minutes without
its re-admission. The first pair maximizes minimum sibling idleness.
All four groups used pair 14,30. Each group is retained whole once
admitted, including every exit. Post-group counters include its own work
and cannot distinguish external interference. No group was replaced.

Publication reduction
Only path spelling and private binding text in harness records changed:
the evidence-directory binding now says admission records in this
evidence set. The executable binding is relative to the target directory;
Bash is named without its installation path. Values, estimates, units,
dates, commits, ordinals, process IDs and counts are unchanged. System
configuration paths in the instruction-probe diagnostic are expressed
as the sysctl key and configuration description; its refusal is retained.
Raw process/service lists, private layout, wrapper invocations, scripts,
probes unrelated to the rows, dry runs, waiters and gate logs are excluded.
Original digests preserve their provenance without publishing those names.
No process/service listing is included in this set.

Limits
These are observations with process variation, not isolated or fixed-
frequency latency. Shared cache, memory bandwidth, package power and
thermal limits remain uncontrolled. The frequency policy is dynamic.
Enabled observation mode is non-comparable. All five estimates and their
minimum, maximum and spread are retained; within-process resampling is
absent and no instruction count is invented. A declaration's consistency
does not establish the runner's history.
