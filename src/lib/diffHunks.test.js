import assert from "node:assert/strict";
import test from "node:test";
import { diffHunks, lineKind, stageableIndex } from "./diffHunks.js";

test("parte el diff en hunks y guarda contexto, altas y bajas", () => {
  const hunks = diffHunks("diff --git a/a b/a\n@@ -1,2 +1,2 @@\n keep\n-old\n+new\n");
  assert.equal(hunks.length, 1);
  assert.equal(hunks[0].header, "@@ -1,2 +1,2 @@");
  assert.deepEqual(hunks[0].lines, [" keep", "-old", "+new"]);
});

test("sin hunks no hay nada que pintar", () => {
  assert.deepEqual(diffHunks(""), []);
  assert.deepEqual(diffHunks("diff --git a/a b/a\n"), []);
});

test("el índice para stage ignora el contexto", () => {
  const lines = [" keep", "-old", " still", "+new"];
  assert.equal(lineKind("-old"), "del");
  assert.equal(stageableIndex(lines, 0), -1);
  assert.equal(stageableIndex(lines, 1), 0);
  assert.equal(stageableIndex(lines, 2), -1);
  assert.equal(stageableIndex(lines, 3), 1);
});
