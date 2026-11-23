export const NOTICE_MS = 3200;
export const ERROR_MS = 7000;
export const BUSY_DELAY_MS = 220;

export function bannerGone(shownAt, now, life) {
  if (!shownAt) return true;
  return now - shownAt >= life;
}

export function busyOverlay(count, waitedMs, delay = BUSY_DELAY_MS) {
  return Number(count) > 0 && waitedMs >= delay;
}
