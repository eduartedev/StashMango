import assert from "node:assert/strict";
import test from "node:test";
import { branchMenuItems, menuPosition } from "./branchMenu.js";

test("la rama actual ofrece red y copiar, no cambiar ni merge", () => {
  const items = branchMenuItems(true);
  assert.deepEqual(items.map((item) => item.id), ["copy", "fetch", "pull", "push"]);
  assert.deepEqual(items.map((item) => item.icon), ["copy", "fetch", "pull", "push"]);
});

test("otra rama ofrece cambiar, merge y rebase, no la red", () => {
  const items = branchMenuItems(false);
  assert.deepEqual(items.map((item) => item.id), ["switch", "merge", "rebase", "copy"]);
  assert.deepEqual(items.map((item) => item.icon), ["switch", "merge", "rebase", "copy"]);
});

test("el menú se queda dentro de la ventana", () => {
  assert.deepEqual(menuPosition(900, 700, 240, 180, { width: 1000, height: 800 }), { x: 752, y: 612 });
  assert.deepEqual(menuPosition(2, -4, 240, 180, { width: 1000, height: 800 }), { x: 8, y: 8 });
});
