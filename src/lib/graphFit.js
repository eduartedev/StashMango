export function graphFit(laneCount, columnWidth) {
  const lanes = Math.max(0, laneCount);
  const width = Math.max(64, Number(columnWidth) || 64);
  const comfort = lanes * 26 + 36;
  const compact = lanes > 0 && width < comfort;
  const gap = compact ? 14 : 26;
  const room = Math.floor((width - 18) / gap);
  const visible = lanes === 0 ? 0 : Math.min(lanes, Math.max(1, room));
  return {
    compact,
    gap,
    visible,
    hidden: Math.max(0, lanes - visible),
    width,
  };
}

export function laneFits(lane, fit) {
  return lane >= 0 && lane < fit.visible;
}
