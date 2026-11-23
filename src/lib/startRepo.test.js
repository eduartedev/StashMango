import assert from "node:assert/strict";
import test from "node:test";
import { assertClone, cloneNameFromUrl, cloneUrlOk, joinDest } from "./startRepo.js";

test("acepta https, ssh y git@, rechaza basura", () => {
  assert.equal(cloneUrlOk("https://github.com/org/repo.git"), true);
  assert.equal(cloneUrlOk("ssh://git@github.com/org/repo.git"), true);
  assert.equal(cloneUrlOk("git@github.com:org/repo.git"), true);
  assert.equal(cloneUrlOk("https://github.com/org/repo.git\n"), false);
  assert.equal(cloneUrlOk("-u id"), false);
  assert.equal(cloneUrlOk("ext::sh -c whoami"), false);
  assert.equal(cloneUrlOk("file:///tmp/repo"), false);
  assert.equal(cloneUrlOk("http://github.com/org/repo.git"), false);
  assert.equal(cloneUrlOk("https://user:pass@github.com/org/repo.git"), false);
});

test("el nombre sale del último tramo, sin .git", () => {
  assert.equal(cloneNameFromUrl("https://github.com/org/repo.git"), "repo");
  assert.equal(cloneNameFromUrl("git@github.com:org/cotizador-web.git"), "cotizador-web");
  assert.equal(cloneNameFromUrl("https://github.com/org/.."), "");
});

test("joinDest usa la barra de la carpeta", () => {
  assert.equal(joinDest("C:\\src", "repo"), "C:\\src\\repo");
  assert.equal(joinDest("/home/edu/src/", "repo"), "/home/edu/src/repo");
  assert.equal(joinDest("C:\\src", ""), "C:\\src");
});

test("assertClone exige URL y destino limpios", () => {
  assert.deepEqual(assertClone("  https://github.com/org/repo.git  ", "C:\\src\\repo"), {
    url: "https://github.com/org/repo.git",
    dest: "C:\\src\\repo",
  });
  assert.throws(() => assertClone("ext::sh", "C:\\src\\repo"), /URL/);
  assert.throws(() => assertClone("https://github.com/org/repo.git", ""), /destino/);
});
