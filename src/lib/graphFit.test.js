import assert from "node:assert/strict";
import test from "node:test";
import { graphFit, laneFits } from "./graphFit.js";

test("con espacio de sobra el grafo usa las líneas completas", () => {
  const fit = graphFit(4, 400);
  assert.equal(fit.compact, false);
  assert.equal(fit.gap, 26);
  assert.equal(fit.hidden, 0);
  assert.equal(laneFits(3, fit), true);
});

test("si la columna es estrecha se aprieta y oculta lo que no cabe", () => {
  const fit = graphFit(12, 120);
  assert.equal(fit.compact, true);
  assert.equal(fit.gap, 14);
  assert.equal(fit.visible >= 1, true);
  assert.equal(fit.hidden, 12 - fit.visible);
  assert.equal(laneFits(0, fit), true);
  assert.equal(laneFits(fit.visible, fit), false);
});

test("sin commits no oculta nada", () => {
  const fit = graphFit(0, 160);
  assert.equal(fit.hidden, 0);
  assert.equal(fit.visible, 0);
});
