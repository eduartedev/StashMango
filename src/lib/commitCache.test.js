import assert from "node:assert/strict";
import test from "node:test";
import { branchStamp, keepLog, restoreLog, shouldReloadLog } from "./commitCache.js";

function node(id, refs = "", lane = 0) {
  return { id, refs, lane, parent_lanes: [lane], summary: id };
}

test("el sello de ramas no depende del orden", () => {
  const a = branchStamp([
    { name: "main", sha: "aa", remote: false },
    { name: "origin/main", sha: "bb", remote: true },
  ]);
  const b = branchStamp([
    { name: "origin/main", sha: "bb", remote: true },
    { name: "main", sha: "aa", remote: false },
  ]);
  assert.equal(a, b);
  assert.notEqual(a, branchStamp([{ name: "main", sha: "cc", remote: false }]));
});

test("si no hay cache, usa la página nueva", () => {
  const page = { commits: [node("a")], open_lanes: ["x"], done: true };
  const next = keepLog({ commits: [], openLanes: [], done: false }, page);
  assert.equal(next.kept, false);
  assert.deepEqual(next.commits, page.commits);
  assert.equal(next.done, true);
});

test("si el prefijo es el mismo, conserva el historial ya cargado", () => {
  const cached = {
    commits: [node("a"), node("b"), node("c")],
    openLanes: ["tail"],
    done: false,
  };
  const page = { commits: [node("a"), node("b")], open_lanes: ["fresh"], done: false };
  const next = keepLog(cached, page);
  assert.equal(next.kept, true);
  assert.equal(next.commits, cached.commits);
  assert.deepEqual(next.openLanes, ["tail"]);
  assert.equal(next.done, false);
});

test("un commit nuevo reemplaza, no deja la lista vacía", () => {
  const cached = { commits: [node("old"), node("b")], openLanes: [], done: true };
  const page = { commits: [node("new"), node("old")], open_lanes: [], done: true };
  const next = keepLog(cached, page);
  assert.equal(next.kept, false);
  assert.equal(next.commits[0].id, "new");
  assert.equal(next.commits.length, 2);
});

test("página vacía no borra lo que ya había", () => {
  const cached = { commits: [node("a")], openLanes: [], done: true };
  const next = keepLog(cached, { commits: [], open_lanes: [], done: true });
  assert.equal(next.kept, true);
  assert.equal(next.commits[0].id, "a");
});

test("restore y skip si el sello no cambió", () => {
  const cache = new Map();
  assert.deepEqual(restoreLog(cache, "/repo").commits, []);
  cache.set("/repo", { commits: [node("a")], openLanes: ["x"], done: true, stamp: "l:main:aa" });
  const hit = restoreLog(cache, "/repo");
  assert.equal(hit.commits[0].id, "a");
  assert.equal(shouldReloadLog(hit, "l:main:aa"), false);
  assert.equal(shouldReloadLog(hit, "l:main:bb"), true);
  assert.equal(shouldReloadLog(restoreLog(cache, "/otro"), "l:main:aa"), true);
});
