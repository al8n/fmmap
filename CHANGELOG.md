# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - unreleased

Rebuilds the safe-API surface around identity, range validation, and
crash-durable I/O after an adversarial review pass. `fmmap`'s safe API is built
over [`memmapix`](https://crates.io/crates/memmapix) 0.9, a rustix-based fork of
`memmap2`.

### Changed (breaking)

- **Lock / unlock methods now take `&mut self`** (`lock`, `lock_shared`,
  `try_lock`, `try_lock_shared`, `unlock`; sync and async). Single-owner
  serialization is now enforced by the borrow checker, and the internal
  `AtomicU8` / `Mutex<u8>` lock-state tracking is gone. Callers that need
  concurrent access wrap the handle in `Arc<Mutex<…>>` themselves.
- The crate moved from `fmmap-rs/` to `fmmap/` within the repository.

### Added

- **Identity-checked deletion.** A `FileIdentity` (POSIX `dev`+`ino`; a
  handle-bound check on Windows) is captured at construction and verified before
  every unlink, so a path that has been reused by a *different* file is refused
  with a tagged error instead of being deleted. Covers `MmapFileMut::remove`,
  the consuming `drop_remove`, the raw `DiskMmapFileMut::drop_remove`, the retry
  path, and the `remove_on_drop` `Drop` path, across sync / tokio / smol.
- **Symlink / reparse-point refusal** at the same syscall as the identity probe
  (POSIX `AT_SYMLINK_NOFOLLOW`; Windows `FILE_FLAG_OPEN_REPARSE_POINT`).
- **Mapping range validation** (`validate_mapping_range`) in every file-backed
  constructor: out-of-bounds or overflowing `offset` / `len` now return
  `ErrorKind::InvalidInput` instead of reaching unchecked pointer arithmetic in
  the backend.
- **Auto-acquired advisory lock** on every constructor — exclusive on writable
  maps, shared on read-only / COW maps; aliased writable mappings of the same
  file are rejected up front.
- **Crash-durability hardening.** The unlink and its parent `fsync` are bound to
  the same parent fd (`fstatat` + `unlinkat` on POSIX) so a concurrent parent
  rename cannot redirect them, and the original open file description is held
  alive across probe + unlink so the kernel cannot recycle `(dev, ino)`
  mid-delete.

- **Recovery-aware raw deletion.** `AsyncDiskMmapFileMut::try_drop_remove`
  returns `DropRemoveError::Recoverable(self, err)` when a pre-destructive fd
  allocation fails under `EMFILE` — the parent-directory handle, or (on smol)
  the inode-pin `fcntl_dupfd_cloexec` — handing `self` back so the caller can
  retry once fd pressure eases. Failures after `self` is destructured are
  `Terminal`. The trait `drop_remove` delegates to it and discards the
  recovered value.

### Notes

- `set_remove_on_drop` auto-cleanup is documented as best-effort; the actual
  unlink is identity-checked.
- Known residual races (the single-syscall POSIX `fstatat`→`unlinkat` TOCTOU
  window, and smol's consuming `drop_remove` under `EMFILE`) are documented in
  the README's Design section.

## Earlier releases

Releases before 0.5.0 (0.1.x – 0.4.x) predate this changelog; see the
[commit history](https://github.com/al8n/fmmap/commits/main) and the
[crates.io version list](https://crates.io/crates/fmmap/versions).

[Unreleased]: https://github.com/al8n/fmmap/commits/main
