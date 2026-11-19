/**
 * Pure mathematical invariants for Watchlist Wilson 95% confidence intervals
 * and bounded simplex factor weight projection (`[minPct, maxPct]`, sum = 100%).
 */

export function computeWilsonBoundsPct(wins: number, total: number): [number, number] {
  if (total <= 0) {
    return [0, 0];
  }
  const safeWins = Math.max(0, Math.min(wins, total));
  const n = total;
  const p = safeWins / n;
  const z = 1.959963984540054;
  const z2 = z * z;
  const denom = 1 + z2 / n;
  const center = (p + z2 / (2 * n)) / denom;
  const halfWidth =
    (z * Math.sqrt((p * (1 - p)) / n + z2 / (4 * n * n))) / denom;
  const lo = Math.max(0, Math.min(100, (center - halfWidth) * 100));
  const hi = Math.max(0, Math.min(100, (center + halfWidth) * 100));
  return [lo, Math.max(lo, hi)];
}

export function normalizeBoundedWeightsPct(
  rawWeights: number[],
  minPct = 5,
  maxPct = 35
): number[] {
  const k = rawWeights.length;
  if (k === 0) return [];
  const effMin = minPct * k > 100 ? 0 : minPct;
  const effMax = maxPct * k < 100 ? 100 : maxPct;
  const sumRaw = rawWeights.reduce((acc, v) => acc + Math.max(0, Number.isFinite(v) ? v : 0), 0);
  const w =
    sumRaw > 1e-9
      ? rawWeights.map((v) =>
          Math.max(effMin, Math.min(effMax, (Math.max(0, Number.isFinite(v) ? v : 0) / sumRaw) * 100))
        )
      : rawWeights.map(() => Math.max(effMin, Math.min(effMax, 100 / k)));

  // Directional water-filling projection onto { sum(w) == 100, effMin <= w_i <= effMax }
  for (let iter = 0; iter < 40; iter += 1) {
    const currentSum = w.reduce((acc, val) => acc + val, 0);
    const diff = 100 - currentSum;
    if (Math.abs(diff) < 1e-9) {
      break;
    }
    const freeIdx: number[] = [];
    for (let i = 0; i < k; i += 1) {
      if (diff > 0 && w[i] < effMax - 1e-9) {
        freeIdx.push(i);
      } else if (diff < 0 && w[i] > effMin + 1e-9) {
        freeIdx.push(i);
      }
    }
    if (freeIdx.length === 0) {
      break;
    }
    const delta = diff / freeIdx.length;
    for (const idx of freeIdx) {
      w[idx] = Math.max(effMin, Math.min(effMax, w[idx] + delta));
    }
  }
  return w;
}
