import assert from "node:assert/strict";
import test from "node:test";
import { canStagePreview, previewImageSrc, previewKicker } from "./previewFile.js";

test("el letrero depende de si es commit, imagen, texto o índice", () => {
  assert.equal(previewKicker({ sha: "abc", kind: "diff" }), "Commit");
  assert.equal(previewKicker({ kind: "image" }), "Imagen");
  assert.equal(previewKicker({ kind: "text" }), "Archivo");
  assert.equal(previewKicker({ cached: true, kind: "diff" }), "En el índice");
  assert.equal(previewKicker({ kind: "diff" }), "Cambios");
});

test("solo el diff del trabajo se puede preparar por hunk", () => {
  assert.equal(canStagePreview({ kind: "diff" }), true);
  assert.equal(canStagePreview({ kind: "diff", sha: "abc" }), false);
  assert.equal(canStagePreview({ kind: "image" }), false);
  assert.equal(canStagePreview({ kind: "text" }), false);
  assert.equal(canStagePreview({ kind: "diff", binary: true }), false);
});

test("la imagen solo se arma con un media de imagen conocido", () => {
  assert.equal(previewImageSrc({ media: "image/png", data: "aaa" }), "data:image/png;base64,aaa");
  assert.equal(previewImageSrc({ media: "text/html", data: "aaa" }), "");
  assert.equal(previewImageSrc({ media: "image/png" }), "");
});
