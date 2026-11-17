import assert from "node:assert/strict";
import test from "node:test";
import { nextSide } from "./sideNav.js";

test("con el panel cerrado, un icono lo abre en esa sección", () => {
  assert.deepEqual(nextSide(false, "ramas", "stash"), { open: true, view: "stash" });
});

test("el mismo icono cierra el panel y deja la sección elegida", () => {
  assert.deepEqual(nextSide(true, "ramas", "ramas"), { open: false, view: "ramas" });
});

test("otro icono cambia la sección y el panel sigue abierto", () => {
  assert.deepEqual(nextSide(true, "ramas", "worktrees"), { open: true, view: "worktrees" });
});

test("un id desconocido abre ramas", () => {
  assert.deepEqual(nextSide(false, "stash", "no-existe"), { open: true, view: "ramas" });
});
