export function branchStamp(refs) {
  return (refs || [])
    .map((ref) => `${ref.remote ? "r" : "l"}:${ref.name}:${ref.sha}`)
    .sort()
    .join("\n");
}

function take(commits, n) {
  const list = commits || [];
  return n == null ? list : list.slice(0, n);
}

export function logIds(commits, n) {
  return take(commits, n)
    .map((commit) => commit.id)
    .join("\n");
}

export function logShape(commits, n) {
  return take(commits, n)
    .map((commit) => `${commit.id}:${commit.refs}:${commit.lane}:${(commit.parent_lanes || []).join(",")}`)
    .join("\n");
}

export function keepLog(cached, page) {
  const have = cached?.commits || [];
  const fresh = page?.commits || [];
  if (!fresh.length) {
    return {
      commits: have,
      openLanes: cached?.openLanes || page?.open_lanes || [],
      done: cached?.done ?? page?.done ?? true,
      kept: have.length > 0,
    };
  }
  if (!have.length) {
    return { commits: fresh, openLanes: page.open_lanes, done: page.done, kept: false };
  }
  const n = fresh.length;
  if (logIds(have, n) === logIds(fresh, n) && logShape(have, n) === logShape(fresh, n) && have.length >= n) {
    return {
      commits: have,
      openLanes: cached.openLanes,
      done: cached.done,
      kept: true,
    };
  }
  return { commits: fresh, openLanes: page.open_lanes, done: page.done, kept: false };
}

export function restoreLog(cache, path) {
  const hit = cache?.get?.(path);
  if (!hit?.commits?.length) {
    return { commits: [], openLanes: [], done: false, stamp: "" };
  }
  return {
    commits: hit.commits,
    openLanes: hit.openLanes || [],
    done: Boolean(hit.done),
    stamp: hit.stamp || "",
  };
}

export function shouldReloadLog(cached, stamp) {
  if (!cached?.commits?.length) return true;
  return (cached.stamp || "") !== stamp;
}
