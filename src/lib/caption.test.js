import assert from "node:assert/strict";
import test from "node:test";
import { captionToggleTarget } from "./caption.js";

function node(className) {
  return { classList: { contains: (name) => name === className } };
}

test("el doble clic en el hueco de la barra restaura o maximiza", () => {
  assert.equal(captionToggleTarget(node("top")), true);
  assert.equal(captionToggleTarget(node("spacer")), true);
});

test("el doble clic en un botón de la barra no cambia el tamaño", () => {
  assert.equal(captionToggleTarget(node("brand")), false);
  assert.equal(captionToggleTarget(node("close")), false);
  assert.equal(captionToggleTarget(node("tab-main")), false);
  assert.equal(captionToggleTarget(null), false);
  assert.equal(captionToggleTarget("top"), false);
});
