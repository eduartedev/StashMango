import assert from "node:assert/strict";
import test from "node:test";
import { branchFlow, branchMenuItems, menuPosition } from "./branchMenu.js";

test("la rama actual ofrece red y copiar, no cambiar ni merge", () => {
  const items = branchMenuItems(true);
  assert.deepEqual(items.map((item) => item.id), ["copy", "fetch", "pull", "push"]);
  assert.deepEqual(items.map((item) => item.icon), ["copy", "fetch", "pull", "push"]);
});

test("otra rama ofrece cambiar, merge y rebase, no la red", () => {
  const items = branchMenuItems(false, "main", "develop");
  assert.deepEqual(items.map((item) => item.id), ["switch", "merge", "rebase", "copy"]);
  assert.deepEqual(items.map((item) => item.icon), ["switch", "merge", "rebase", "copy"]);
  assert.equal(items.find((item) => item.id === "switch").flow, "develop → main");
  assert.equal(items.find((item) => item.id === "merge").flow, "main → develop");
  assert.equal(items.find((item) => item.id === "rebase").flow, "develop → main");
});

test("si faltan nombres, el flujo habla de esta rama y la actual", () => {
  assert.equal(branchFlow("", ""), "esta rama → rama actual");
  const items = branchMenuItems(false);
  assert.equal(items.find((item) => item.id === "merge").flow, "esta rama → rama actual");
});

test("el menú se queda dentro de la ventana", () => {
  assert.deepEqual(menuPosition(900, 700, 240, 180, { width: 1000, height: 800 }), { x: 752, y: 612 });
  assert.deepEqual(menuPosition(2, -4, 240, 180, { width: 1000, height: 800 }), { x: 8, y: 8 });
});
