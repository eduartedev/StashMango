import assert from "node:assert/strict";
import test from "node:test";
import { branchRows, localBranchNames, matchesBranch } from "./branchList.js";

const refs = [
  { name: "develop", sha: "aaa", remote: false },
  { name: "main", sha: "bbb", remote: false },
  { name: "feature/foo", sha: "ccc", remote: false },
  { name: "origin/develop", sha: "aaa", remote: true },
  { name: "origin/main", sha: "ddd", remote: true },
  { name: "origin/feature/foo", sha: "ccc", remote: true },
  { name: "origin/HEAD", sha: "bbb", remote: true },
  { name: "origin/solo", sha: "eee", remote: true },
  { name: "upstream/develop", sha: "aaa", remote: true },
];

test("por defecto solo se ven las carpetas local y remotas, cerradas", () => {
  const rows = branchRows(refs);
  assert.deepEqual(
    rows.map((row) => [row.kind, row.path, row.shut, row.count]),
    [
      ["folder", "local", true, 3],
      ["folder", "origin", true, 2],
      ["folder", "upstream", true, 1],
    ],
  );
  assert.deepEqual(localBranchNames(refs), ["develop", "main", "feature/foo"]);
});

test("al abrir local sin gitflow, feature sigue colapsado", () => {
  const rows = branchRows(refs, { local: false });
  assert.deepEqual(
    rows.filter((row) => row.kind === "folder").map((row) => [row.path, row.shut, row.count]),
    [
      ["local", false, 3],
      ["local/feature", true, 1],
      ["origin", true, 2],
      ["upstream", true, 1],
    ],
  );
  assert.deepEqual(
    rows.filter((row) => row.kind === "leaf").map((row) => [row.branch, row.synced]),
    [
      ["develop", true],
      ["main", false],
    ],
  );
});

test("gitflow parte local en production, develop y feature", () => {
  const rows = branchRows(refs, { local: false }, { gitflow: true });
  assert.deepEqual(
    rows.filter((row) => row.path?.startsWith("local")).map((row) => [row.path, row.shut, row.count]),
    [
      ["local", false, 3],
      ["local/production", true, 1],
      ["local/develop", true, 1],
      ["local/feature", true, 1],
    ],
  );
  const open = branchRows(refs, { local: false, "local/feature": false }, { gitflow: true });
  assert.deepEqual(
    open.filter((row) => row.kind === "leaf").map((row) => [row.branch, row.label]),
    [["feature/foo", "foo"]],
  );
});

test("gitflow respeta ramas sueltas configuradas", () => {
  const rows = branchRows(
    [
      { name: "trunk", sha: "aaa", remote: false },
      { name: "cotizador-web", sha: "bbb", remote: false },
      { name: "wip/login", sha: "ccc", remote: false },
    ],
    { local: false, "local/production": false, "local/feature": false },
    { gitflow: { enabled: true, production: "trunk", feature: "cotizador-web, wip/" } },
  );
  assert.deepEqual(
    rows.filter((row) => row.kind === "leaf").map((row) => [row.branch, row.label]),
    [
      ["trunk", "trunk"],
      ["cotizador-web", "cotizador-web"],
      ["wip/login", "login"],
    ],
  );
  assert.equal(rows.find((row) => row.path === "local/production").count, 1);
  assert.equal(rows.find((row) => row.path === "local/feature").count, 2);
  assert.equal(rows.some((row) => row.path === "local/develop"), false);
});

test("origin abierto oculta las que ya coinciden con local", () => {
  const rows = branchRows(refs, { origin: false });
  assert.deepEqual(
    rows.filter((row) => row.kind === "leaf").map((row) => row.branch),
    ["origin/main", "origin/solo"],
  );
  assert.equal(rows.some((row) => row.branch === "origin/develop"), false);
  assert.equal(rows.some((row) => row.branch === "origin/HEAD"), false);
});

test("la búsqueda abre las carpetas y deja solo las que coinciden", () => {
  const rows = branchRows(refs, {}, { query: "FOO" });
  assert.deepEqual(
    rows.filter((row) => row.kind === "leaf").map((row) => row.branch),
    ["feature/foo"],
  );
  assert.equal(rows.find((row) => row.path === "local").shut, false);
  assert.equal(rows.find((row) => row.path === "local/feature").shut, false);
  assert.equal(rows.some((row) => row.branch === "develop"), false);
  assert.equal(rows.some((row) => row.path === "origin"), false);
});

test("matchesBranch ignora mayúsculas y el vacío no filtra", () => {
  assert.equal(matchesBranch("feature/login", "LOG"), true);
  assert.equal(matchesBranch("main", "  "), true);
  assert.equal(matchesBranch("main", "dev"), false);
});

test("si origin solo repite la local, no aparece la carpeta", () => {
  const rows = branchRows([
    { name: "develop", sha: "aaa", remote: false },
    { name: "origin/develop", sha: "aaa", remote: true },
  ]);
  assert.deepEqual(
    rows.map((row) => [row.kind, row.path, row.shut]),
    [["folder", "local", true]],
  );
});
