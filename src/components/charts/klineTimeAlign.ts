import type { Time, UTCTimestamp } from "lightweight-charts";
import type {
  KlineBarDto,
  KlineSignalMarkerDto,
  StockKlinePayloadDto,
} from "../../types/terminal";

export interface AlignedKlineBar extends KlineBarDto {
  chartTime: Time;
}

export interface AlignedKlineMarker {
  chartTime: Time;
  marker: KlineSignalMarkerDto;
}

export function parseBarChartTime(barTime: string): Time {
  const trimmed = barTime.trim();
  if (trimmed.includes(" ")) {
    const normalized = trimmed.replace(" ", "T");
    // Support both "YYYY-MM-DDTHH:mm" (16 chars) and "YYYY-MM-DDTHH:mm:ss" (19+ chars)
    const timePart = normalized.split("T")[1] ?? "";
    const colonCount = (timePart.match(/:/g) ?? []).length;
    const iso = normalized.endsWith("Z")
      ? normalized
      : colonCount >= 2
        ? `${normalized}Z`
        : `${normalized}:00Z`;
    const ms = Date.parse(iso);
    if (Number.isFinite(ms)) {
      return Math.floor(ms / 1000) as UTCTimestamp;
    }
  }
  return trimmed.slice(0, 10) as Time;
}

function compareChartTimes(a: Time, b: Time): number {
  if (typeof a === "number" && typeof b === "number") {
    return a - b;
  }
  return String(a).localeCompare(String(b));
}

export function normalizeKlineSeriesAndMarkers(payload: StockKlinePayloadDto): {
  bars: AlignedKlineBar[];
  markers: AlignedKlineMarker[];
} {
  const rawAligned: AlignedKlineBar[] = [];
  for (const b of payload.bars) {
    const chartTime = parseBarChartTime(b.time);
    if (typeof chartTime === "number" && !Number.isFinite(chartTime)) {
      continue;
    }
    rawAligned.push({ ...b, chartTime });
  }

  rawAligned.sort((a, b) => compareChartTimes(a.chartTime, b.chartTime));

  // Deduplicate bars sharing the same chartTime (keep latest bar update)
  const dedupBars: AlignedKlineBar[] = [];
  for (const bar of rawAligned) {
    const last = dedupBars[dedupBars.length - 1];
    if (last && String(last.chartTime) === String(bar.chartTime)) {
      dedupBars[dedupBars.length - 1] = bar;
    } else {
      dedupBars.push(bar);
    }
  }

  if (dedupBars.length === 0) {
    return { bars: [], markers: [] };
  }

  const isIntraday = payload.period === "30m" || typeof dedupBars[0].chartTime === "number";
  const markerMap = new Map<string, AlignedKlineMarker>();

  for (const m of payload.buyMarkers) {
    const rawTime = m.barTime.trim();
    const datePrefix = rawTime.slice(0, 10);
    let snappedTime: Time | null = null;

    if (isIntraday) {
      if (rawTime.includes(" ")) {
        const parsed = parseBarChartTime(rawTime);
        const exact = dedupBars.find((b) => b.chartTime === parsed);
        if (exact) {
          snappedTime = exact.chartTime;
        }
      }
      if (snappedTime === null) {
        const sameDayBar = dedupBars.find((b) => b.time.slice(0, 10) === datePrefix);
        if (sameDayBar) {
          snappedTime = sameDayBar.chartTime;
        }
      }
    } else {
      const exactOrPrior = [...dedupBars]
        .reverse()
        .find((b) => b.time.slice(0, 10) <= datePrefix);
      if (exactOrPrior && datePrefix >= dedupBars[0].time.slice(0, 10)) {
        snappedTime = exactOrPrior.chartTime;
      }
    }

    if (snappedTime !== null) {
      markerMap.set(String(snappedTime), {
        chartTime: snappedTime,
        marker: m,
      });
    }
  }

  const markers = Array.from(markerMap.values()).sort((a, b) =>
    compareChartTimes(a.chartTime, b.chartTime)
  );

  return { bars: dedupBars, markers };
}
