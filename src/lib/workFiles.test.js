import assert from "node:assert/strict";
import test from "node:test";
import { fileFolder, fileName, groupWorkFiles } from "./workFiles.js";

test("el nombre es el archivo y la carpeta el resto", () => {
  assert.equal(fileName(".github/workflows/deploy.yml"), "deploy.yml");
  assert.equal(fileFolder(".github/workflows/deploy.yml"), ".github/workflows");
  assert.equal(fileName(".gitignore"), ".gitignore");
  assert.equal(fileFolder(".gitignore"), "");
  assert.equal(fileName(""), "");
});

test("los archivos se agrupan por conflicto, índice, cambios y sin seguimiento", () => {
  const groups = groupWorkFiles([
    { path: "a.txt", staged: true, unstaged: false, untracked: false, conflict: false },
    { path: "b.txt", staged: false, unstaged: true, untracked: false, conflict: false },
    { path: "c.txt", staged: true, unstaged: true, untracked: false, conflict: false },
    { path: "d.txt", staged: false, unstaged: false, untracked: true, conflict: false },
    { path: "e.txt", staged: false, unstaged: false, untracked: false, conflict: true },
  ]);
  assert.deepEqual(groups.map((group) => [group.id, group.files.map((file) => file.path)]), [
    ["conflict", ["e.txt"]],
    ["staged", ["a.txt", "c.txt"]],
    ["unstaged", ["b.txt", "c.txt"]],
    ["untracked", ["d.txt"]],
  ]);
});

test("sin archivos no hay grupos", () => {
  assert.deepEqual(groupWorkFiles([]), []);
  assert.deepEqual(groupWorkFiles(undefined), []);
});
