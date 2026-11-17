import assert from "node:assert/strict";
import test from "node:test";
import { sameWorktreePath, worktreeFolder, worktreeLabel } from "./worktreeCard.js";

test("la carpeta es el último tramo, con barras de Windows o Unix", () => {
  assert.equal(worktreeFolder("C:\\laragon\\www\\Perfiletto-cotizador-v2"), "Perfiletto-cotizador-v2");
  assert.equal(worktreeFolder("/home/edu/repos/Perfiletto"), "Perfiletto");
  assert.equal(worktreeFolder(""), "");
});

test("el título usa la rama y, suelto, el hash corto", () => {
  assert.equal(worktreeLabel({ branch: "feature/cotizador-v2", head: "abcdef123" }), "feature/cotizador-v2");
  assert.equal(worktreeLabel({ branch: "", head: "abcdef123456" }), "abcdef1");
  assert.equal(worktreeLabel(null), "");
});

test("el worktree actual coincide aunque las barras cambien", () => {
  const tree = "C:\\laragon\\www\\Perfiletto";
  assert.equal(sameWorktreePath(tree, "C:/laragon/www/Perfiletto"), true);
  assert.equal(sameWorktreePath(tree, "C:\\laragon\\www\\otro"), false);
  assert.equal(sameWorktreePath("", "C:\\laragon\\www\\Perfiletto"), false);
});
