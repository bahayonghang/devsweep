import { useEffect, useState } from "react";
import type { HudStatusEvent, StatusSnapshotV1 } from "../api/types.gen";
import { formatBytes } from "../components/format";
import {
  message,
  type MessageKey,
  type PresentationLanguageTag,
} from "../i18n";
import {
  availableValue,
  formatBasisPoints,
  formatTenths,
  memoryBasisPoints,
  peakGpuBasisPoints,
  peakTemperatureTenths,
} from "../modes/status/state";

export type HudSubscribe = (
  onEvent: (event: HudStatusEvent) => void,
  onEventError: (error: unknown) => void,
) => Promise<() => void>;

export interface HudRow {
  readonly key: string;
  readonly label: string;
  readonly value: string;
}

/** Compact HUD rows. A missing value shows as unavailable, never as zero. */
export function hudRows(
  locale: PresentationLanguageTag,
  snapshot: StatusSnapshotV1,
): HudRow[] {
  const unavailable = message(locale, "hud.v1.unavailable");
  const percent = (points: number | null) =>
    points === null ? unavailable : `${formatBasisPoints(points)}%`;
  const cpu = availableValue(snapshot.cpu);
  const temperature = peakTemperatureTenths(snapshot);
  const rows: HudRow[] = [
    {
      key: "cpu",
      label: message(locale, "hud.v1.cpu"),
      value: percent(cpu ? cpu.system_utilization_basis_points : null),
    },
    {
      key: "memory",
      label: message(locale, "hud.v1.memory"),
      value: percent(memoryBasisPoints(snapshot)),
    },
    {
      key: "gpu",
      label: message(locale, "hud.v1.gpu"),
      value: percent(peakGpuBasisPoints(snapshot)),
    },
    {
      key: "temperature",
      label: message(locale, "hud.v1.temperature"),
      value:
        temperature === null ? unavailable : `${formatTenths(temperature)} °C`,
    },
  ];

  const volumes = availableValue(snapshot.volumes);
  if (volumes && volumes.items.length > 0) {
    for (const volume of volumes.items) {
      rows.push({
        key: `disk-${volume.volume_id}`,
        label: message(locale, "hud.v1.disk"),
        value: message(locale, "status.v1.volume.item", {
          mount: volume.mount_points[0] ?? "-",
          available: formatBytes(volume.available_bytes),
          total: formatBytes(volume.total_bytes),
        }),
      });
    }
  } else {
    rows.push({
      key: "disk",
      label: message(locale, "hud.v1.disk"),
      value: unavailable,
    });
  }

  const network = availableValue(snapshot.network);
  rows.push({
    key: "network",
    label: message(locale, "hud.v1.network"),
    value: network
      ? message(locale, "hud.v1.network.value", {
          rx: formatBytes(
            network.interfaces.reduce(
              (sum, item) => sum + item.rx_bytes_per_second,
              0,
            ),
          ),
          tx: formatBytes(
            network.interfaces.reduce(
              (sum, item) => sum + item.tx_bytes_per_second,
              0,
            ),
          ),
        })
      : unavailable,
  });

  const power = availableValue(snapshot.power);
  if (!power) {
    rows.push({
      key: "battery",
      label: message(locale, "hud.v1.battery"),
      value: unavailable,
    });
  } else if (power.battery_present) {
    const parts = [
      percent(power.charge_basis_points),
      message(locale, "status.v1.power.ac", {
        ac: message(locale, `status.v1.ac.${power.ac_state}` as MessageKey),
      }),
    ];
    if (power.remaining_seconds !== null) {
      parts.push(
        message(locale, "status.v1.stage.battery.remaining", {
          minutes: String(Math.floor(power.remaining_seconds / 60)),
        }),
      );
    }
    rows.push({
      key: "battery",
      label: message(locale, "hud.v1.battery"),
      value: parts.join(" · "),
    });
  }
  return rows;
}

export function Hud({
  locale,
  subscribe,
}: {
  readonly locale: PresentationLanguageTag;
  readonly subscribe: HudSubscribe;
}) {
  const [snapshot, setSnapshot] = useState<StatusSnapshotV1 | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    subscribe(
      (event) => setSnapshot(event.type === "snapshot" ? event.snapshot : null),
      // The closed decoder rejects the payload; the next valid sample
      // replaces the view.
      () => undefined,
    )
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribe]);

  return (
    <main className="hud" aria-label={message(locale, "hud.v1.title")}>
      <h1 className="hud-title">{message(locale, "hud.v1.title")}</h1>
      {snapshot === null ? (
        <p className="hud-sampling" role="status">
          {message(locale, "hud.v1.sampling")}
        </p>
      ) : (
        <dl className="hud-rows">
          {hudRows(locale, snapshot).map((row) => (
            <div className="hud-row" key={row.key}>
              <dt>{row.label}</dt>
              <dd>{row.value}</dd>
            </div>
          ))}
        </dl>
      )}
    </main>
  );
}
