import assert from "node:assert/strict";
import { createServer } from "node:net";
import { describe, it } from "node:test";
import { DEV_PORT_ENV, devServerPortFromEnv, findLoopbackPort } from "./dev-server-port.mjs";
import { devUrlConfig, shouldSelectDevPort, withDevConfig } from "./tauri-dev.mjs";

function listen(port) {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}

function close(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

describe("dev server port", () => {
  it("reads an explicit port and rejects values that are not TCP ports", () => {
    assert.equal(devServerPortFromEnv({}, 4180), 4180);
    assert.equal(devServerPortFromEnv({ [DEV_PORT_ENV]: "" }, 4180), 4180);
    assert.equal(devServerPortFromEnv({ [DEV_PORT_ENV]: "4182" }, 4180), 4182);
    assert.throws(() => devServerPortFromEnv({ [DEV_PORT_ENV]: "0" }, 4180), /TCP port/);
    assert.throws(() => devServerPortFromEnv({ [DEV_PORT_ENV]: "65536" }, 4180), /TCP port/);
    assert.throws(() => devServerPortFromEnv({ [DEV_PORT_ENV]: "4180 " }, 4180), /TCP port/);
  });

  it("returns the preferred port when it can listen", async () => {
    const preferred = await findLoopbackPort(45000, 30);
    assert.equal(await findLoopbackPort(preferred, 5), preferred);
  });

  it("skips a busy preferred port", async () => {
    const preferred = await findLoopbackPort(46000, 30);
    const held = await listen(preferred);
    try {
      const found = await findLoopbackPort(preferred, 20);
      assert.ok(found > preferred);
    } finally {
      await close(held);
    }
  });
});

describe("tauri dev arguments", () => {
  it("selects a port only for a real dev run", () => {
    assert.equal(shouldSelectDevPort(["dev"]), true);
    assert.equal(shouldSelectDevPort(["dev", "--release"]), true);
    assert.equal(shouldSelectDevPort(["build"]), false);
    assert.equal(shouldSelectDevPort(["dev", "--help"]), false);
    assert.equal(shouldSelectDevPort(["dev", "--", "--help"]), true);
  });

  it("places the devUrl override before runner arguments", () => {
    const configPath = "C:\\temp\\dev-url.json";
    assert.deepEqual(withDevConfig(["dev", "--release", "--", "--flag"], configPath), [
      "dev",
      "--release",
      "--config",
      configPath,
      "--",
      "--flag",
    ]);
    assert.deepEqual(devUrlConfig(4182), {
      build: { devUrl: "http://127.0.0.1:4182" },
    });
  });
});
