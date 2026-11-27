import assert from "node:assert/strict";
import test from "node:test";
import { settingsNav, settingsSectionOr } from "./settingsNav.js";

test("la barra de configuración tiene secciones con icono", () => {
  assert.deepEqual(settingsNav.map((item) => item.id), ["perfiles", "servicios", "ssh", "plantillas", "git", "columnas", "apariencia"]);
  assert.equal(new Set(settingsNav.map((item) => item.icon)).size, 7);
});

test("una sección desconocida vuelve a perfiles", () => {
  assert.equal(settingsSectionOr("ssh"), "ssh");
  assert.equal(settingsSectionOr("no-existe"), "perfiles");
});
