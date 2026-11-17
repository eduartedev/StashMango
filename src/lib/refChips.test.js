import assert from "node:assert/strict";
import test from "node:test";
import { chipKind, isCurrentRef, packRefs, refChips, refTarget } from "./refChips.js";

test("separa HEAD, la rama, la etiqueta y el remoto", () => {
  const chips = refChips("HEAD -> main, tag: 1.6.1.0, origin/main, origin/HEAD");
  assert.deepEqual(
    chips.map((chip) => [chip.kind, chip.label]),
    [
      ["head", "HEAD"],
      ["local", "main"],
      ["tag", "1.6.1.0"],
      ["remote", "origin/HEAD"],
    ],
  );
  assert.equal(chips.find((chip) => chip.label === "main").synced, true);
});

test("la local y origin/main se juntan; si el remoto va solo, se queda", () => {
  const together = refChips("origin/develop, develop");
  assert.deepEqual(together.map((chip) => [chip.kind, chip.label]), [["local", "develop"]]);
  assert.equal(together[0].synced, true);
  const apart = refChips("origin/develop");
  assert.deepEqual(apart.map((chip) => [chip.kind, chip.label]), [["remote", "origin/develop"]]);
  assert.equal(apart[0].synced, undefined);
});

test("varias etiquetas dejan una visible y el resto aparte", () => {
  const chips = refChips("HEAD -> main, tag: 1.6.1.0, origin/main");
  const packed = packRefs(chips);
  assert.equal(packed.shown.label, "main");
  assert.deepEqual(packed.extra.map((chip) => chip.label), ["1.6.1.0", "HEAD"]);
});

test("solo una rama local o su remoto se pueden elegir", () => {
  const branches = ["develop", "main"];
  assert.equal(refTarget({ kind: "local", label: "develop" }, branches), "develop");
  assert.equal(refTarget({ kind: "remote", label: "origin/develop" }, branches), "develop");
  assert.equal(refTarget({ kind: "remote", label: "origin/HEAD" }, branches), "");
  assert.equal(refTarget({ kind: "tag", label: "1.6.1.0" }, branches), "");
  assert.equal(chipKind("tag"), "Etiqueta");
});

test("la palomita marca la rama actual y, suelto, solo HEAD", () => {
  const chips = refChips("HEAD -> main, tag: 1.6.1.0, origin/main, origin/HEAD");
  const marked = chips.filter((chip) => isCurrentRef(chip, "main")).map((chip) => chip.label);
  assert.deepEqual(marked, ["main"]);
  assert.equal(isCurrentRef({ kind: "remote", label: "origin/main" }, "main"), false);
  const loose = chips.filter((chip) => isCurrentRef(chip, "")).map((chip) => chip.kind);
  assert.deepEqual(loose, ["head"]);
});

test("sin referencias no hay pastillas", () => {
  assert.deepEqual(refChips(""), []);
  assert.deepEqual(refChips(undefined), []);
});
