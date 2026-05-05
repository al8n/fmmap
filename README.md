<div align="center">
<h1>fmmap</h1>
</div>
<div align="center">

A flexible and convenient high-level mmap for zero-copy file I/O.

[<img alt="github" src="https://img.shields.io/badge/github-al8n/fmmap-8da0cb?style=for-the-badge&logo=Github" height="22">][Github-url]
<img alt="LoC" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fgist.githubusercontent.com%2Fal8n%2F327b2a8aef9003246e45c6e47fe63937%2Fraw%2Ffmmap" height="22">
[<img alt="Build" src="https://img.shields.io/github/actions/workflow/status/al8n/fmmap/ci.yml?logo=Github-Actions&style=for-the-badge" height="22">][CI-url]
[<img alt="codecov" src="https://img.shields.io/codecov/c/gh/al8n/fmmap?style=for-the-badge&token=6R3QFWRWHL&logo=codecov" height="22">][codecov-url]

[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-fmmap-66c2a5?style=for-the-badge&labelColor=555555&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">][doc-url]
[<img alt="crates.io" src="https://img.shields.io/crates/v/fmmap?style=for-the-badge&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iaXNvLTg4NTktMSI/Pg0KPCEtLSBHZW5lcmF0b3I6IEFkb2JlIElsbHVzdHJhdG9yIDE5LjAuMCwgU1ZHIEV4cG9ydCBQbHVnLUluIC4gU1ZHIFZlcnNpb246IDYuMDAgQnVpbGQgMCkgIC0tPg0KPHN2ZyB2ZXJzaW9uPSIxLjEiIGlkPSJMYXllcl8xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIiB4PSIwcHgiIHk9IjBweCINCgkgdmlld0JveD0iMCAwIDUxMiA1MTIiIHhtbDpzcGFjZT0icHJlc2VydmUiPg0KPGc+DQoJPGc+DQoJCTxwYXRoIGQ9Ik0yNTYsMEwzMS41MjgsMTEyLjIzNnYyODcuNTI4TDI1Niw1MTJsMjI0LjQ3Mi0xMTIuMjM2VjExMi4yMzZMMjU2LDB6IE0yMzQuMjc3LDQ1Mi41NjRMNzQuOTc0LDM3Mi45MTNWMTYwLjgxDQoJCQlsMTU5LjMwMyw3OS42NTFWNDUyLjU2NHogTTEwMS44MjYsMTI1LjY2MkwyNTYsNDguNTc2bDE1NC4xNzQsNzcuMDg3TDI1NiwyMDIuNzQ5TDEwMS44MjYsMTI1LjY2MnogTTQzNy4wMjYsMzcyLjkxMw0KCQkJbC0xNTkuMzAzLDc5LjY1MVYyNDAuNDYxbDE1OS4zMDMtNzkuNjUxVjM3Mi45MTN6IiBmaWxsPSIjRkZGIi8+DQoJPC9nPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPC9zdmc+DQo=" height="22">][crates-url]
[<img alt="crates.io" src="https://img.shields.io/crates/d/fmmap?color=critical&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBzdGFuZGFsb25lPSJubyI/PjwhRE9DVFlQRSBzdmcgUFVCTElDICItLy9XM0MvL0RURCBTVkcgMS4xLy9FTiIgImh0dHA6Ly93d3cudzMub3JnL0dyYXBoaWNzL1NWRy8xLjEvRFREL3N2ZzExLmR0ZCI+PHN2ZyB0PSIxNjQ1MTE3MzMyOTU5IiBjbGFzcz0iaWNvbiIgdmlld0JveD0iMCAwIDEwMjQgMTAyNCIgdmVyc2lvbj0iMS4xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHAtaWQ9IjM0MjEiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkzIiB3aWR0aD0iNDgiIGhlaWdodD0iNDgiIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIj48ZGVmcz48c3R5bGUgdHlwZT0idGV4dC9jc3MiPjwvc3R5bGU+PC9kZWZzPjxwYXRoIGQ9Ik00NjkuMzEyIDU3MC4yNHYtMjU2aDg1LjM3NnYyNTZoMTI4TDUxMiA3NTYuMjg4IDM0MS4zMTIgNTcwLjI0aDEyOHpNMTAyNCA2NDAuMTI4QzEwMjQgNzgyLjkxMiA5MTkuODcyIDg5NiA3ODcuNjQ4IDg5NmgtNTEyQzEyMy45MDQgODk2IDAgNzYxLjYgMCA1OTcuNTA0IDAgNDUxLjk2OCA5NC42NTYgMzMxLjUyIDIyNi40MzIgMzAyLjk3NiAyODQuMTYgMTk1LjQ1NiAzOTEuODA4IDEyOCA1MTIgMTI4YzE1Mi4zMiAwIDI4Mi4xMTIgMTA4LjQxNiAzMjMuMzkyIDI2MS4xMkM5NDEuODg4IDQxMy40NCAxMDI0IDUxOS4wNCAxMDI0IDY0MC4xOTJ6IG0tMjU5LjItMjA1LjMxMmMtMjQuNDQ4LTEyOS4wMjQtMTI4Ljg5Ni0yMjIuNzItMjUyLjgtMjIyLjcyLTk3LjI4IDAtMTgzLjA0IDU3LjM0NC0yMjQuNjQgMTQ3LjQ1NmwtOS4yOCAyMC4yMjQtMjAuOTI4IDIuOTQ0Yy0xMDMuMzYgMTQuNC0xNzguMzY4IDEwNC4zMi0xNzguMzY4IDIxNC43MiAwIDExNy45NTIgODguODMyIDIxNC40IDE5Ni45MjggMjE0LjRoNTEyYzg4LjMyIDAgMTU3LjUwNC03NS4xMzYgMTU3LjUwNC0xNzEuNzEyIDAtODguMDY0LTY1LjkyLTE2NC45MjgtMTQ0Ljk2LTE3MS43NzZsLTI5LjUwNC0yLjU2LTUuODg4LTMwLjk3NnoiIGZpbGw9IiNmZmZmZmYiIHAtaWQ9IjM0MjIiIGRhdGEtc3BtLWFuY2hvci1pZD0iYTMxM3guNzc4MTA2OS4wLmkwIiBjbGFzcz0iIj48L3BhdGg+PC9zdmc+&style=for-the-badge" height="22">][crates-url]
<img alt="license" src="https://img.shields.io/badge/License-Apache%202.0/MIT-blue.svg?style=for-the-badge&fontColor=white&logoColor=f5c076&logo=data:image/svg+xml;base64,PCFET0NUWVBFIHN2ZyBQVUJMSUMgIi0vL1czQy8vRFREIFNWRyAxLjEvL0VOIiAiaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkIj4KDTwhLS0gVXBsb2FkZWQgdG86IFNWRyBSZXBvLCB3d3cuc3ZncmVwby5jb20sIFRyYW5zZm9ybWVkIGJ5OiBTVkcgUmVwbyBNaXhlciBUb29scyAtLT4KPHN2ZyBmaWxsPSIjZmZmZmZmIiBoZWlnaHQ9IjgwMHB4IiB3aWR0aD0iODAwcHgiIHZlcnNpb249IjEuMSIgaWQ9IkNhcGFfMSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB4bWxuczp4bGluaz0iaHR0cDovL3d3dy53My5vcmcvMTk5OS94bGluayIgdmlld0JveD0iMCAwIDI3Ni43MTUgMjc2LjcxNSIgeG1sOnNwYWNlPSJwcmVzZXJ2ZSIgc3Ryb2tlPSIjZmZmZmZmIj4KDTxnIGlkPSJTVkdSZXBvX2JnQ2FycmllciIgc3Ryb2tlLXdpZHRoPSIwIi8+Cg08ZyBpZD0iU1ZHUmVwb190cmFjZXJDYXJyaWVyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiLz4KDTxnIGlkPSJTVkdSZXBvX2ljb25DYXJyaWVyIj4gPGc+IDxwYXRoIGQ9Ik0xMzguMzU3LDBDNjIuMDY2LDAsMCw2Mi4wNjYsMCwxMzguMzU3czYyLjA2NiwxMzguMzU3LDEzOC4zNTcsMTM4LjM1N3MxMzguMzU3LTYyLjA2NiwxMzguMzU3LTEzOC4zNTcgUzIxNC42NDgsMCwxMzguMzU3LDB6IE0xMzguMzU3LDI1OC43MTVDNzEuOTkyLDI1OC43MTUsMTgsMjA0LjcyMywxOCwxMzguMzU3UzcxLjk5MiwxOCwxMzguMzU3LDE4IHMxMjAuMzU3LDUzLjk5MiwxMjAuMzU3LDEyMC4zNTdTMjA0LjcyMywyNTguNzE1LDEzOC4zNTcsMjU4LjcxNXoiLz4gPHBhdGggZD0iTTE5NC43OTgsMTYwLjkwM2MtNC4xODgtMi42NzctOS43NTMtMS40NTQtMTIuNDMyLDIuNzMyYy04LjY5NCwxMy41OTMtMjMuNTAzLDIxLjcwOC0zOS42MTQsMjEuNzA4IGMtMjUuOTA4LDAtNDYuOTg1LTIxLjA3OC00Ni45ODUtNDYuOTg2czIxLjA3Ny00Ni45ODYsNDYuOTg1LTQ2Ljk4NmMxNS42MzMsMCwzMC4yLDcuNzQ3LDM4Ljk2OCwyMC43MjMgYzIuNzgyLDQuMTE3LDguMzc1LDUuMjAxLDEyLjQ5NiwyLjQxOGM0LjExOC0yLjc4Miw1LjIwMS04LjM3NywyLjQxOC0xMi40OTZjLTEyLjExOC0xNy45MzctMzIuMjYyLTI4LjY0NS01My44ODItMjguNjQ1IGMtMzUuODMzLDAtNjQuOTg1LDI5LjE1Mi02NC45ODUsNjQuOTg2czI5LjE1Miw2NC45ODYsNjQuOTg1LDY0Ljk4NmMyMi4yODEsMCw0Mi43NTktMTEuMjE4LDU0Ljc3OC0zMC4wMDkgQzIwMC4yMDgsMTY5LjE0NywxOTguOTg1LDE2My41ODIsMTk0Ljc5OCwxNjAuOTAzeiIvPiA8L2c+IDwvZz4KDTwvc3ZnPg==" height="22">

</div>

## Design
Inspired by Dgraph's mmap file implementation in [ristretto](https://github.com/hypermodeinc/ristretto).

A file-backed memory map exposes the kernel's view of an inode as a `&[u8]`/`&mut [u8]`. That makes it easy to reach for, but it also means UB the moment another actor truncates, unlinks, or rewrites the file out from under the mapping — SIGBUS on Unix, mapping detachment on Windows, silent torn reads in either. `fmmap` raises a safe API over `memmapix` by treating those concerns as first-class:

- **Auto-acquired advisory lock** on every constructor — exclusive on writable maps, shared on read-only / COW maps. Aliased writable mappings of the same file (and mut-then-COW) are rejected up front.
- **Best-effort path-reuse mitigation on deletion**. Identity is captured at open and re-checked before every unlink so a file someone else has swapped in at the path won't be silently deleted. POSIX uses `(st_dev, st_ino)`; Windows uses `(volumeSerial, fileIndex)` from `GetFileInformationByHandle` (via `windows-sys`, no nightly required). **This is not an absolute guarantee** — see the [path-reuse limitations](#path-reuse-limitations) below.
- **Pre-validated mapping ranges**. Constructors reject `offset`/`len` overflow, ranges past EOF, and effective lengths > `isize::MAX` *before* any destructive `set_len` runs, so an invalid `Options` never zeroes or extends an existing file.
- **Crash-durable unlink**. The parent directory is pinned by a handle opened *before* `remove_file`, then fsynced through that same handle. Failed-fsync retries fsync the *same* handle (not a freshly-opened parent), so a parent rename between unlink and fsync can't direct the durability to the wrong inode.
- **Reentrant-safe lock methods**. `LockFileEx` deadlocks on the same Windows handle; `lock` / `lock_shared` short-circuit when the desired state is already held. The lock methods take `&mut self` so single-owner serialization is enforced by the borrow checker.
- **Poison-safe truncate / freeze**. A failed truncate marks the wrapper poisoned; subsequent reads return `&[]` and writes/flushes/freezes return `Err` rather than handing back an anonymous-mapped placeholder pretending to be the original file.

`std` plus tokio and smol are first-class. The async surface is built from the same set of macros, so adding a new runtime is small and mechanical — see `fmmap/src/disk/{tokio,smol}_impl.rs`.

### What identity-checked delete actually guarantees

Identity-checked deletion is built on the strongest atomic primitives each platform exposes; what's left is a small, documented set of irreducible races.

**POSIX**: probe + unlink + parent fsync are all bound to the same parent fd via `rustix`'s `fstatat` + `unlinkat`. A parent rename mid-operation can't direct the unlink or fsync to a different directory than the one we verified. The original file's open-file description is held alive (via `fcntl(F_DUPFD_CLOEXEC)` or, in the tokio wrapper, `tokio::fs::File::into_std()`) across probe + unlink, so the kernel cannot recycle `(dev, ino)` to a fresh file in the window. Identity capture itself is allocation-free (`fstat` on a `BorrowedFd`), so EMFILE has no path to defeating the identity check.

**Windows**: probe and unlink are bound to a single handle. The handle is opened with `DELETE | FILE_SHARE_*` and `FILE_FLAG_OPEN_REPARSE_POINT`; we re-verify identity and refuse reparse points on that handle, then issue `SetFileInformationByHandle(FileDispositionInfoEx)` with `POSIX_SEMANTICS | IGNORE_READONLY_ATTRIBUTE`. Older Windows / FAT32 fall back to `FileDispositionInfo` after a `ReOpenFile` widens access to clear `FILE_ATTRIBUTE_READONLY` (using `FILE_ATTRIBUTE_NORMAL` as the cleared-state sentinel — Windows treats `0` as "no change"). Identity is captured directly via `GetFileInformationByHandle` on a borrowed `HANDLE` — no `DuplicateHandle`, no fd alloc.

**API contract**: explicit `remove()` (and `drop_remove()`) only returns `Ok` if fmmap itself observed the unlink succeed in the parent it then fsynced. `NotFound` from the probe or unlink is *never* converted into a durable-success retry — the wrapper stays in `NeedsUnlink` and surfaces the error, even when the inode's `nlink` has dropped to 0 (which can't distinguish "unlink in our parent" from "external rename + unlink elsewhere"). Drop's best-effort cleanup still fsyncs the parent in the common case, but the API doesn't promise durability we can't verify.

### Residual races (irreducible at this layer)

- **One-syscall TOCTOU on POSIX.** Between `fstatat` and `unlinkat` — both bound to the same parent fd — there's still a single-syscall window where the entry could be replaced. Closing this needs an inode-bound `unlinkat` primitive POSIX doesn't expose. The window is dramatically narrower than the handle-drop-to-retry window the identity check *does* close, but it's not zero.
- **External rename + unlink elsewhere.** A concurrent actor can rename our file into a different directory and unlink it there. The inode's `nlink` drops to 0 but our parent's fsync doesn't commit *their* unlink. fmmap detects this only as "the file is gone" and surfaces `NotFound`; under that scenario, callers who need crash-durability should serialize external mutations or `fsync` the relevant parents themselves.
- **Smol consuming `drop_remove(self)` under EMFILE.** smol's `async-fs::File` exposes no `into_std()`, so the inode pin is a `fcntl_dupfd_cloexec` of the underlying fd. Under fd pressure the dup fails, `drop_remove` returns `Err` deterministically (no hidden Drop-time retry), and the file remains on disk. Callers can recover via `std::fs::remove_file(path)` directly or `AsyncMmapFileMut::remove(&mut self)` which preserves `self` for an explicit retry. Tokio's `into_std()` allocates no fd so this limitation doesn't apply on tokio.

If your threat model includes an active local adversary, do not rely on identity-checked delete for safety — perform the cleanup yourself with whatever atomic primitives your platform provides.

## Features
- [x] file-backed memory maps with auto-locked construction
- [x] read-only / copy-on-write / mutable / executable maps
- [x] identity-checked deletion bound to a single kernel-verified handle (POSIX `fstatat`+`unlinkat` on a parent fd; Windows `SetFileInformationByHandle(FileDispositionInfoEx)` on a `DELETE | FILE_SHARE_DELETE` handle); see [Design](#design) for residual races
- [x] inode pin across probe + unlink (POSIX `F_DUPFD_CLOEXEC` or tokio `into_std`) — defends against `(dev, ino)` recycling on tmpfs / small-id filesystems
- [x] crash-durable unlink with pre-opened parent fsync (same handle reused on retry)
- [x] symlink / reparse-point refusal at the same syscall as the identity probe (POSIX `AT_SYMLINK_NOFOLLOW`, Windows `FILE_FLAG_OPEN_REPARSE_POINT`)
- [x] readonly-file delete on Windows (`FileDispositionInfoEx` with `IGNORE_READONLY_ATTRIBUTE`, legacy `FileDispositionInfo` fallback for pre-1607)
- [x] pre-validated mapping ranges (rejects past-EOF and `> isize::MAX` before any destructive `set_len`)
- [x] poison-safe `truncate` / `freeze` / `freeze_exec`
- [x] synchronous and asynchronous flushing
- [x] reader / writer adapters with byteorder + seek
- [x] dozens of file I/O util functions
- [x] stack support (`MAP_STACK` on Unix)
- [x] [tokio][tokio]
- [x] [smol][smol]

## Installation

`fmmap` requires Rust **1.75** or later.

- std
    ```toml
    [dependencies]
    fmmap = "0.5"
    ```

- [tokio][tokio]
    ```toml
    [dependencies]
    fmmap = { version = "0.5", default-features = false, features = ["tokio"] }
    ```

- [smol][smol]
    ```toml
    [dependencies]
    fmmap = { version = "0.5", default-features = false, features = ["smol"] }
    ```

The `sync` feature is on by default.

## Examples
This crate is 100% documented, see [docs.rs][doc-url] for examples.

## License

<sup>
Licensed under either of <a href="https://opensource.org/licenses/Apache-2.0">Apache License, Version
2.0</a> or <a href="https://opensource.org/licenses/MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>


[Github-url]: https://github.com/al8n/fmmap/
[CI-url]: https://github.com/al8n/fmmap/actions/workflows/ci.yml
[doc-url]: https://docs.rs/fmmap
[crates-url]: https://crates.io/crates/fmmap
[codecov-url]: https://app.codecov.io/gh/al8n/fmmap/
[license-url]: https://opensource.org/licenses/Apache-2.0
[rustc-url]: https://github.com/rust-lang/rust/blob/master/RELEASES.md
[license-apache-url]: https://opensource.org/licenses/Apache-2.0
[license-mit-url]: https://opensource.org/licenses/MIT
[tokio]: https://crates.io/crates/tokio 
[smol]: https://crates.io/crates/smol
