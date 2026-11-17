import assert from "node:assert/strict";
import test from "node:test";
import {
  defaultTemplates,
  fillTemplate,
  filesList,
  insertToken,
  normalizeTemplates,
  sampleVars,
  scopeFrom,
  templateKinds,
  ticketFrom,
  typeFrom,
} from "./commitTemplates.js";

test("la plantilla de merge rellena la rama que entra y la actual", () => {
  const text = fillTemplate(defaultTemplates.merge, { branch: "feature/login", current: "develop" });
  assert.equal(text, "Merge branch 'feature/login' into develop");
});

test("si falta un valor, se limpia el hueco y los corchetes vacíos", () => {
  assert.equal(fillTemplate("Merge {branch} → {current}", { branch: "main" }), "Merge main →");
  assert.equal(fillTemplate("[{ticket}] {summary}", { summary: "arregla el login" }), "arregla el login");
  assert.equal(fillTemplate("", { branch: "main" }), "");
});

test("sin archivo se usan los textos por defecto", () => {
  const templates = normalizeTemplates();
  assert.equal(templates.merge, defaultTemplates.merge);
  assert.equal(templates.stash, defaultTemplates.stash);
  assert.equal(templates.cherry, defaultTemplates.cherry);
  assert.equal(templates.commit, "");
  assert.equal(normalizeTemplates({ merge: "", commit: "WIP: " }).merge, "");
  assert.equal(templateKinds.map((item) => item.id).join(","), "commit,merge,stash,cherry,rebase,reword,pr");
});

test("el ticket, el tipo y el ámbito salen de la rama y las rutas", () => {
  assert.equal(ticketFrom("feature/JIRA-12-login"), "JIRA-12");
  assert.equal(typeFrom("feature/login"), "feat");
  assert.equal(typeFrom("fix-boton"), "fix");
  assert.equal(scopeFrom(["Construccion/a.css", "Construccion/b.js"]), "Construccion");
  assert.equal(filesList(["a/uno.css", "a/dos.js", "a/tres.html", "a/cuatro.md"]), "uno.css, dos.js, tres.html +1");
});

test("insertar un hueco respeta el cursor", () => {
  assert.deepEqual(insertToken("Merge  into develop", "branch", 6, 6), {
    text: "Merge {branch} into develop",
    cursor: 14,
  });
});

test("el ejemplo de la mesa de plantillas tiene ticket y archivos", () => {
  const vars = sampleVars({ branch: "develop" }, []);
  assert.equal(vars.ticket, "JIRA-12");
  assert.equal(vars.type, "feat");
  assert.equal(vars.scope, "Construccion");
  assert.match(fillTemplate(defaultTemplates.pr, vars), /JIRA-12/);
});
