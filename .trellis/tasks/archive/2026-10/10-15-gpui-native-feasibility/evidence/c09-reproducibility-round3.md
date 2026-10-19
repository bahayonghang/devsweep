# C09 release-build reproducibility re-test (round 3)

Date: 2026-10-18. Same source tree, same locked dependencies, same toolchain
(rustc/cargo 1.99.0), no `incremental` line and no `warning:` line in any build
log.

## Result: the release build is NOT bit-for-bit reproducible on this host

| Experiment | Build 1 SHA-256 | Build 2 SHA-256 | Identical |
| --- | --- | --- | --- |
| Two different target dirs (`dist/repro-a`, `dist/repro-b`) | `9C192A32FF2D2C225D2C058B5053BA244294FD5A05CFF866F07F7A67DB1D1AC7` | `FB060EFA3C31AD0DE0B1B471B226DA1BE5936EF2EAE1F85A1300F3E03A2BE8D9` | **no** |
| Same target dir, deleted and rebuilt twice (`dist/repro-c`) | `34CB6934BA0B70C6173CA455510BD0B5DCE11E639CF54D0DC73E2DA12B03D8B6` | `147F68BEECABE7561AD1F90F4E19249D806170ECF4E213CFB8ADE75D10F23444` | **no** |
| Two more fresh dirs (`dist/d1`, `dist/d2`) | `9CBCA7A56E80FE5C714BCB46A691BA63E69E846B970705DFB7F4459034690F8A` | `9A82AED1A0C73D06C38A8726C414F79304563CB59550053BD94658561C9D378F` | **no** |

All builds exited 0. Every binary was exactly 14179328 bytes, so the size is
stable while the content is not.

## Where the bytes differ

Comparing `dist/d1` against `dist/d2`, exactly 24 bytes differ:

```
offsets: 248, 249, 12296388, 12296389, 12296416, 12296417, 12296444, 12296445,
         12296820 .. 12296835
```

Offsets 248-249 sit inside the MSVC **Rich header** immediately after the
`Rich` marker (the surrounding bytes read `Rich.>??....PE..d?...`). The bytes at
~12.3 MB are in a late data section and include an eight-byte run plus the
aligned block around it, which is where the PE debug/build-id material lives.

That signature — a handful of bytes in the Rich header and the timestamp/build-id
region, with everything else identical — is the classic MSVC linker
non-determinism (PE `TimeDateStamp` and Rich-header build info), not a source or
dependency difference.

## Consequence for C09-AC3

C09-AC3 asks that the pinned build be reproducible. This run **fails** the
bit-identical reading of that clause on this host. Earlier rounds recorded
matching SHA-256 pairs; those matches are not reproducible now, so they cannot
be used as a standing reproducibility guarantee. Either the acceptance wording
means "same lock, same compiler, same inputs builds successfully" (which does
hold) or it means bit-identical output (which does not).

This is recorded as a FAIL for the bit-identical reading and is not retried away
or reclassified. A real fix would be to make the link step reproducible — for
example passing `/Brepro` through `RUSTFLAGS` and confirming the Rich header and
PE timestamp stop varying — and that is a build-configuration change that has
not been made.

## Reproduce

```powershell
cd desktop-gpui
cargo build --locked --release --target-dir dist/d1
cargo build --locked --release --target-dir dist/d2
Get-FileHash dist/d1/release/devsweep-gpui-spike.exe -Algorithm SHA256
Get-FileHash dist/d2/release/devsweep-gpui-spike.exe -Algorithm SHA256
```

## Resolution: `/Brepro` makes the link deterministic

The failure above was diagnosed and fixed. The MSVC linker is not deterministic
by default; `/Brepro` removes the Rich-header build info and the PE timestamp
variation. `desktop-gpui` has no way to pass link arguments through its own
manifest, so the flag is supplied as
`RUSTFLAGS='-C link-arg=/Brepro'`.

Verified through the committed `just gpui-build` recipe — two clean builds into
separate fresh target directories:

| Build | SHA-256 | Bytes | `incremental` lines | `warning:` lines |
| --- | --- | --- | --- | --- |
| `dist/rp1` | `DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34` | 14179328 | 0 | 0 |
| `dist/rp2` | `DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34` | 14179328 | 0 | 0 |

Identical. A third pair (`dist/bp1`, `dist/bp2`) reproduced the same hash
`DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34`, so this is
stable across four builds, not a coincidence.

The recipe is now in `justfile` as `gpui-check`, `gpui-test`, `gpui-build` and
the aggregate `gpui`. `just gpui` exits 0. This also closes the earlier
observation that `desktop-gpui` had no first-class gate recipe at all, so the
root `ci` could never cover it.

C09-AC3's bit-identical reproducibility clause is now satisfied **with the
`/Brepro` flag in place**. Builds made without it remain non-reproducible, and
that earlier receipt stays recorded above rather than being deleted.
