# Roadmap

The engine grows in stages, each a new runtime or capability around the same core, as the
decision *Runtimes proven against a sequential reference* requires. A stage keeps every law,
states what it expects, and lands only when its benchmark confirms it. A new crate, such as an
async runtime or a disk store, needs its own decision record first.

## Benchmarks

- Inputs: a seeded benchmark generator under `backend/examples/`, so the package keeps one
  binary for `cargo run`. It writes 1M, 10M and 100M rows: 50% deposits, 35% withdrawals,
  8% disputes, 4% resolves, 3% chargebacks; clients across the `u16` range; padded and unpadded
  fields.
- Metrics: rows per second and peak resident memory, measured with the `time` utility, for
  `--release` and for the dev profile that `cargo run` builds.
- Method: one machine, best of five runs, recorded below with the commit.
- Gates, by kind of stage:
  - every stage: all laws pass, and the output equals the sequential reference's;
  - speed-up: beats the previous stage on 10M rows by its stated factor;
  - enabling: trails the previous stage by at most 5%;
  - capability: meets its own targets and states the throughput it costs;
  - peak memory stays within 10% of the previous stage unless the stage names the extra.

## Results

| Stage | Commit | Rows | Profile | Rows/s | Peak memory |
|---|---|---|---|---|---|

*(none yet)*

## 1. Sequential reference (now)

- Adds: the domain types, the core, the store ports with in-memory stores, the sequential `run`,
  the CSV adapters and the report's text adapter, the laws, the model and the fixtures; `make run`
  takes an input file.
- Expects: bound by decoding. A throwaway probe decoded 2.5M to 4.4M rows/s on one thread with
  `csv` and `serde`.
- Follow-ups, each kept only if measured faster:
  - dev profile `opt-level`: the probe's debug build took 12 to 21 s per 5M rows; dependencies
    at `opt-level = 3` brought that to 6 to 8 s, and our crate at `opt-level = 1` to under 3 s;
  - one `ByteRecord` reused across rows: up to 1.4× faster decoding in the probe;
  - `with_capacity` for the stores; a paged bitmap for `SeenIds`, at most 512 MiB for every
    `u32`; packed deposit records, 42 to 28 bytes per entry in the probe.
- Open: a bound on one row's size. `csv` grows its record buffer for any row; a bound needs a
  reader below `csv`, and a record of its own.

## 2. Partitioned workers (enabling)

- Adds: an intake thread (`SeenIds`, then routing by `client % N`); N workers, each owning one
  partition's stores; bounded queues carrying batches of commands; accounts merged from the
  partitions. Threads come from `std::thread::scope`.
- Keeps: output independent of N.
- Expects: at most about 1.3× on one file, since decoding still runs on one thread. It enables
  stages 3 and 4.
- Gate: at most 5% slower than stage 1, at any N.

## 3. Parallel decoding (speed-up)

- Adds: a block reader (blocks cut at newlines), M decoders, and a sequencer that restores block
  order before intake. Cutting at newlines rules out quoted fields that contain one; that limit
  needs its own design record before the stage lands.
- Keeps: output independent of M and of block size, down to a few bytes. A failed decoder stops
  the run with an error, never a hang.
- Expects: decoding scales with M until intake or the workers saturate.
- Gate: at least 2× stage 2 on 10M rows.

## 4. Service runtime (capability)

- Adds: an async network edge; one source per connection; one intake ordering the rows of every
  connection, so first sight follows intake order; long-lived partition workers fed by bounded
  queues.
- Order: per client, intake order. A sender that needs a stricter order keeps each client on one
  connection.
- Measures: throughput and per-batch latency (p50, p99) with 1,000 open connections; memory per
  connection under backpressure.
- Gate: throughput per core within 10% of stage 3; memory bounded under load.

## 5. Durability and horizontal scale (capability)

- Adds: a per-partition log of events and of the ids intake consumed, with snapshots; recovery
  folds `evolve` over the log; a disk-backed `LedgerStore`; a partition map across nodes,
  rebalanced by replay.
- Measures: append rate with batched fsync; recovery rows/s; rebalance time; history memory at
  2^32 deposits.
- Gate: recovery reproduces the state exactly at every crash point the tests inject; the stage
  states its throughput cost.
- Open: tx id uniqueness across nodes: an upstream contract, or `SeenIds` partitioned by tx id,
  which would supersede the single intake of *State partitioned by client* and needs its own
  record.
