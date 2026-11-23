import assert from "node:assert/strict";
import test from "node:test";
import { parseStash, stashRefOk, stashRows } from "./stashList.js";

test("parsea el listado de git stash", () => {
  const rows = stashRows([
    "stash@{0}: On feature/add-jd78-jd17-13p: WIP on feature/add-jd78-jd17-13p",
    "stash@{2}: On feature/cotizador-v2: graphify temp",
    "stash@{4}: On develop: Auto stash before merge of \"develop\" and \"feature/cotizador-general-vigas\"",
    "stash@{5}: WIP on hotfix/error-bpi12: limita rangos",
    "basura",
  ]);
  assert.equal(rows.length, 4);
  assert.equal(rows[0].id, "stash@{0}");
  assert.equal(rows[0].branch, "feature/add-jd78-jd17-13p");
  assert.equal(rows[1].title, "graphify temp");
  assert.equal(rows[2].branch, "develop");
  assert.equal(rows[3].branch, "hotfix/error-bpi12");
  assert.equal(rows[3].title, "limita rangos");
});

test("stashRefOk solo acepta stash@{n}", () => {
  assert.equal(stashRefOk("stash@{0}"), true);
  assert.equal(stashRefOk("stash@{12}"), true);
  assert.equal(stashRefOk("stash@{0};rm"), false);
  assert.equal(stashRefOk("stash@{-1}"), false);
  assert.equal(stashRefOk("refs/stash"), false);
  assert.equal(parseStash("stash@{0}: hola\nstash@{1}: x"), null);
});
