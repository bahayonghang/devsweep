// The managed sandbox denies Node child-process creation. Vite's Windows path
// optimizer runs the optional `net use` probe even for local drives. Return the
// same error branch Vite already handles so configLoader=runner can test the UI
// without esbuild or external processes.
const childProcess = require("node:child_process");
const { syncBuiltinESMExports } = require("node:module");

const originalExec = childProcess.exec;
childProcess.exec = function sandboxExec(command, ...args) {
  if (command === "net use") {
    const callback = args.at(-1);
    queueMicrotask(() => callback?.(new Error("sandbox: optional net use probe disabled"), "", ""));
    return undefined;
  }
  return originalExec.call(this, command, ...args);
};
syncBuiltinESMExports();
