# Pinned Tauri channel retention

Read-only dependency-source inspection after the corrected B3 gate. This is a
mechanism finding, not a measured native queue length or latency result.

`Cargo.lock` pins Tauri 2.11.5. The source inspected is the cached crate's
`src/ipc/channel.rs`, with registration in `src/app.rs`, plus installed
`desktop/node_modules/@tauri-apps/api/core.js`.

## Confirmed mechanism

- JSON channel payloads smaller than 8192 bytes go directly through webview
  evaluation. Larger payloads enter `ChannelDataIpcQueue`, an app-managed
  mutex-protected `HashMap<u32, InvokeResponseBody>` without a capacity check.
- `channel_on` inserts the serialized body, then evaluates a script invoking
  `plugin:__TAURI_CHANNEL__|fetch` with the `Tauri-Channel-Id` header. A successful
  `Channel::send` does not wait for JavaScript receipt or decode.
- The fetch handler removes the stored body before JavaScript dispatch. Fetch
  is transport retrieval, not an application-level acknowledgment of decoding
  or reducer acceptance.
- Channel drop emits an end marker. This path does not purge stored bodies for
  a renderer that never retrieved them. The storage is private to Tauri, so
  application code cannot directly report its size through the public type.
- The JavaScript channel preserves index order and keeps out-of-order messages
  in an unbounded sparse array until missing earlier messages arrive.

Source anchors in this pinned crate: JSON threshold `channel.rs:39`, queue
`channel.rs:46`, pre-enqueue interceptor `channel.rs:147`, large-body insertion
and fetch script `channel.rs:168–183`, drop end marker `channel.rs:190`, fetch
removal `channel.rs:319–339`, and app-level registration `app.rs:2411`.
Installed API callback ordering and buffering are at `core.js:95–113`.

## Consequence and measurement boundary

A single raw pending slot plus a 100 ms publication interval bounds the
application forwarder's retention. It does not bound accumulated Tauri/renderer
messages during a paused consumer. This possibility must be measured before
claiming RT-AC4's downstream behavior.

The native driver can observe real scan IPC, pause the renderer, retain native
resource samples, then count sequence-indexed callbacks and bytes after resume.
Outstanding channel-fetch requests can be observed separately if CDP supports
request-stage interception for the native custom protocol. That models delayed
transport and must be labeled separately from a frozen renderer. Neither method
directly reads the private Rust map.

Tauri's public `Builder::channel_interceptor` can observe each body/index before
enqueue in a separately labeled instrumented build, but it is not required for
ordinary product behavior or uninstrumented resource measurements. Do not add a
general telemetry service or treat instrumentation timing as release CPU data.

If the native experiment demonstrates growing retained backlog, the approved
runtime design allows narrow typed flow control. Preserve per-input validation,
phase/final delivery, cancellation/join, closed frontend decoding and final-only
promotion. A cap that silently discards lifecycle or terminal events would not
satisfy that contract.
