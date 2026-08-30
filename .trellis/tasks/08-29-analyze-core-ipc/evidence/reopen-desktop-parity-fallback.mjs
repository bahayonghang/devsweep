import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { decodeAnalyzeSnapshot } from "../../../../desktop/src/api/contract.ts";
import {
  createAnalyzeIndex,
  selectDirectory,
  selectVisibleChildren,
} from "../../../../desktop/src/modes/analyze/selectors.ts";

const fixtureUrl = new URL(
  "../../../../desktop/src/api/fixtures/analyze/cli-parity.json",
  import.meta.url,
);
const envelope = JSON.parse(readFileSync(fixtureUrl, "utf8"));
const snapshotHash = createHash("sha256")
  .update(JSON.stringify(envelope.data))
  .digest("hex");
const snapshot = decodeAnalyzeSnapshot(envelope.data);
const index = createAnalyzeIndex(snapshot);
const root = selectDirectory(index, 0);
const visible = selectVisibleChildren(root.children, "", "size_desc");
const summary = {
  nodes: snapshot.nodes.length,
  root_bytes: root.directory.bytes,
  visible_bytes: visible.reduce((bytes, node) => bytes + node.bytes, 0),
  complete: snapshot.nodes.filter((node) => node.evidence === "complete").length,
  incomplete: snapshot.nodes.filter((node) => node.evidence === "incomplete").length,
  access_denied: snapshot.warnings.filter(
    (warning) => warning.class === "access_denied",
  ).length,
  snapshot_sha256: snapshotHash,
};

assert.deepEqual(summary, {
  nodes: 7,
  root_bytes: 8_388_616,
  visible_bytes: 8_388_616,
  complete: 5,
  incomplete: 2,
  access_denied: 1,
  snapshot_sha256:
    "d14a6c33214dbb4a19527b6184411b68b02b90bec639fb9d8c94e08f7a3e1c36",
});

process.stdout.write(`${JSON.stringify(summary)}\n`);
