import assert from "node:assert/strict";
import test from "node:test";
import { placeRepoTab } from "./repoTabs.js";

const parent = "C:\\laragon\\www\\Perfiletto";
const child = "C:\\laragon\\www\\Perfiletto-cotizador-v2";
const other = "C:\\laragon\\www\\Otro";
const family = [
  { path: parent, branch: "main" },
  { path: child, branch: "feature/cotizador-v2" },
];

test("un worktree reutiliza el tab del padre y conserva su nombre", () => {
  const tabs = placeRepoTab([], { path: parent, branch: "main" }, family);
  const next = placeRepoTab(tabs, { path: child, branch: "feature/cotizador-v2" }, family);
  assert.equal(next.length, 1);
  assert.equal(next[0].id, tabs[0].id);
  assert.equal(next[0].name, "Perfiletto");
  assert.equal(next[0].path, child);
  assert.equal(next[0].branch, "feature/cotizador-v2");
});

test("tabs hermanos del mismo repo se funden en el del padre", () => {
  const tabs = [
    { id: "otro", path: other, name: "Otro", branch: "main" },
    { path: parent, name: "Perfiletto", branch: "main" },
    { path: child, name: "Perfiletto-cotizador-v2", branch: "feature/cotizador-v2" },
  ];
  const next = placeRepoTab(tabs, { path: child, branch: "feature/cotizador-v2" }, family);
  assert.deepEqual(next.map((tab) => tab.path), [other, child]);
  assert.equal(next[1].name, "Perfiletto");
});

test("un repo distinto sigue abriendo su propio tab", () => {
  const tabs = placeRepoTab([], { path: parent, branch: "main" }, family);
  const next = placeRepoTab(tabs, { path: other, branch: "develop" }, family);
  assert.equal(next.length, 2);
  assert.equal(next[1].name, "Otro");
  assert.equal(next[1].path, other);
});

test("sin lista de worktrees cada carpeta es su propio tab", () => {
  const tabs = placeRepoTab([], { path: parent, branch: "main" }, []);
  const next = placeRepoTab(tabs, { path: child, branch: "feature/cotizador-v2" }, []);
  assert.equal(next.length, 2);
});
