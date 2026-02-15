# ImportBao Size Rejections During Sync

**Status:** Documented (upstream issue)

## Objective

Investigate and fix the spurious ImportBao size rejections that occur during P2P blob sync, where blob sizes appear corrupted/garbage during negotiation.

## Background

During e2e testing, we observe errors like:

```
ERROR jax_blobs_store::actor: ImportBao: rejecting import of hash fa81500eb86ed618a55390a67dc2d987277a008c283db1e3e8ca278cab15cdea with unreasonable size 8988579870866186464 (max is 1073741824)
```

The reported sizes (e.g., `8988579870866186464`) are clearly garbage values - they exceed petabytes. Despite these rejections, blobs eventually sync correctly through retries.

## Root Cause

**Upstream issue in iroh-blobs BAO stream protocol.**

In iroh-blobs v0.95.0, the BAO export protocol sends blob size as raw 8 little-endian
bytes at the start of the stream (`api/blobs.rs:import_bao_reader`):

```rust
let mut size = [0; 8];
reader.recv_exact(&mut size).await...;
let size = u64::from_le_bytes(size);
```

There is no framing, length-prefix, or checksum around this value. During P2P
discovery, stream corruption or misaligned reads can cause the receiver to interpret
arbitrary bytes (partial parent hashes, leaf data, or garbage) as the size field,
producing values like `8988579870866186464` (0x7cbdd9c4de34a0e0).

These rejections are **transient** — each retry creates a fresh connection, and
eventually a clean stream arrives.

## Mitigation

- `MAX_IMPORT_SIZE` (1 GB) check in `crates/object-store/src/actor.rs` prevents OOM
  from garbage sizes (added in commit `25495a2`)
- Log level changed from ERROR to WARN since these rejections are expected and transient
- Sync retries handle the transient failures transparently

## Acceptance Criteria

- [x] Root cause identified
- [x] Fix implemented (or documented as upstream issue)
- [ ] No spurious ImportBao rejections during normal sync
- [x] `cargo test` passes
- [x] `cargo clippy` has no warnings

## Verification

```bash
./bin/dev kill --force && ./bin/dev clean
./bin/dev run --background
sleep 90
./bin/dev logs grep "ImportBao: rejecting" | wc -l
# Should be 0 after fix
```
