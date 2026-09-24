import net from "node:net";

export const DEV_SERVER_HOST = "127.0.0.1";
export const PREFERRED_DEV_PORT = 4180;
export const DEV_PORT_ENV = "DEVSWEEP_DESKTOP_DEV_PORT";

export function devServerPortFromEnv(env, fallback = PREFERRED_DEV_PORT) {
  const raw = env[DEV_PORT_ENV];
  if (raw === undefined || raw === "") {
    return fallback;
  }
  if (!/^[1-9][0-9]*$/.test(raw)) {
    throw new Error(`${DEV_PORT_ENV} must be an integer TCP port, got ${raw}`);
  }
  const port = Number(raw);
  if (port > 65535) {
    throw new Error(`${DEV_PORT_ENV} must be an integer TCP port, got ${raw}`);
  }
  return port;
}

function canListen(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    let settled = false;
    const finish = (value) => {
      if (settled) {
        return;
      }
      settled = true;
      resolve(value);
    };
    server.once("error", () => {
      finish(false);
    });
    server.listen(port, DEV_SERVER_HOST, () => {
      server.close((error) => {
        finish(!error);
      });
    });
  });
}

export async function findLoopbackPort(start, attempts) {
  let checked = 0;
  for (let port = start; checked < attempts && port <= 65535; port += 1) {
    checked += 1;
    if (await canListen(port)) {
      return port;
    }
  }
  throw new Error(`No free ${DEV_SERVER_HOST} port from ${start} after ${checked} candidates.`);
}
