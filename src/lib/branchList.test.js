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

test("por defecto se agrupa en LOCAL y cada remoto como secciones", () => {
  const sections = branchRows(refs);
  assert.deepEqual(
    sections.map((s) => [s.id, s.label]),
    [
      ["local", "LOCAL"],
      ["origin", "ORIGIN"],
      ["upstream", "UPSTREAM"],
    ],
  );
  assert.deepEqual(localBranchNames(refs), ["develop", "main", "feature/foo"]);
});

test("gitflow agrupa dentro de LOCAL y ORIGIN", () => {
  const sections = branchRows(refs, {}, { gitflow: true });
  assert.deepEqual(
    sections.map((s) => [s.id, s.label]),
    [
      ["local", "LOCAL"],
      ["origin", "ORIGIN"],
      ["upstream", "UPSTREAM"],
    ],
  );
  const localSection = sections.find((s) => s.id === "local");
  assert.equal(localSection.rows.length, 3);
  assert.equal(localSection.rows[0].name, "Production");
  assert.equal(localSection.rows[0].depth, 0);
  assert.equal(localSection.rows[1].name, "Develop");
  assert.equal(localSection.rows[2].name, "Feature");
});

test("sin gitflow, local muestra ramas y subcarpetas directamente", () => {
  const sections = branchRows(refs, {}, { });
  const localSection = sections.find((s) => s.id === "local");
  assert.deepEqual(
    localSection.rows.filter((row) => row.kind === "folder").map((row) => [row.path, row.shut, row.count]),
    [
      ["feature", true, 1],
    ],
  );
  assert.deepEqual(
    localSection.rows.filter((row) => row.kind === "leaf").map((row) => [row.branch, row.synced]),
    [
      ["develop", true],
      ["main", false],
    ],
  );
});

test("gitflow respeta ramas sueltas configuradas", () => {
  const sections = branchRows(
    [
      { name: "trunk", sha: "aaa", remote: false },
      { name: "cotizador-web", sha: "bbb", remote: false },
      { name: "wip/login", sha: "ccc", remote: false },
    ],
    { local: false, "local/production": false, "local/feature": false },
    { gitflow: { enabled: true, production: "trunk", feature: "cotizador-web, wip/" } },
  );
  const localSection = sections.find((s) => s.id === "local");
  assert.deepEqual(
    localSection.rows.filter((row) => row.kind === "leaf").map((row) => [row.branch, row.label]),
    [
      ["trunk", "trunk"],
      ["cotizador-web", "cotizador-web"],
      ["wip/login", "login"],
    ],
  );
  assert.equal(localSection.rows.find((row) => row.path === "local/production").count, 1);
  assert.equal(localSection.rows.find((row) => row.path === "local/feature").count, 2);
  assert.equal(localSection.rows.some((row) => row.path === "local/develop"), false);
});

test("origin abierto oculta las que ya coinciden con local", () => {
  const sections = branchRows(refs, { origin: false });
  const originSection = sections.find((s) => s.id === "origin");
  assert.deepEqual(
    originSection.rows.filter((row) => row.kind === "leaf").map((row) => row.branch),
    ["origin/main", "origin/solo"],
  );
  assert.equal(originSection.rows.some((row) => row.branch === "origin/develop"), false);
  assert.equal(originSection.rows.some((row) => row.branch === "origin/HEAD"), false);
});

test("la búsqueda abre las carpetas y deja solo las que coinciden", () => {
  const sections = branchRows(refs, {}, { query: "FOO" });
  const flat = sections.flatMap((s) => s.rows);
  assert.deepEqual(
    flat.filter((row) => row.kind === "leaf").map((row) => row.branch),
    ["feature/foo"],
  );
  const localSection = sections.find((s) => s.id === "local");
  assert.equal(localSection.rows.find((row) => row.path === "feature").shut, false);
  assert.equal(sections.some((s) => s.id === "origin"), false);
});

test("matchesBranch ignora mayúsculas y el vacío no filtra", () => {
  assert.equal(matchesBranch("feature/login", "LOG"), true);
  assert.equal(matchesBranch("main", "  "), true);
  assert.equal(matchesBranch("main", "dev"), false);
});

test("si origin solo repite la local, no aparece la carpeta", () => {
  const sections = branchRows([
    { name: "develop", sha: "aaa", remote: false },
    { name: "origin/develop", sha: "aaa", remote: true },
  ]);
  assert.deepEqual(
    sections.map((s) => [s.id, s.label]),
    [["local", "LOCAL"]],
  );
});
