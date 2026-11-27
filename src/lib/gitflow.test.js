import assert from "node:assert/strict";
import test from "node:test";
import { assertGitflow, defaultGitflow, gitflowBucket, gitflowCreateKinds, gitflowKinds, gitflowTokenOk, normalizeGitflow, parseGitflowTokens } from "./gitflow.js";

test("los valores por defecto son los de git flow", () => {
  const flow = defaultGitflow();
  assert.equal(flow.enabled, false);
  assert.equal(flow.production, "main, master");
  assert.equal(flow.develop, "develop");
  assert.equal(flow.feature, "feature/");
  assert.deepEqual(gitflowKinds.map((item) => item.id), ["production", "develop", "feature", "release", "hotfix", "support"]);
});

test("normalizeGitflow rellena huecos y acepta true", () => {
  assert.equal(normalizeGitflow(true).enabled, true);
  assert.equal(normalizeGitflow(true).feature, "feature/");
  assert.equal(normalizeGitflow({ enabled: true, production: "  trunk ,  " }).production, "trunk");
  assert.equal(normalizeGitflow({ feature: "" }).feature, "feature/");
});

test("el cubo por defecto pone main en production y feature/foo en feature", () => {
  assert.equal(gitflowBucket("main").id, "production");
  assert.equal(gitflowBucket("master").id, "production");
  assert.equal(gitflowBucket("develop").id, "develop");
  assert.deepEqual(gitflowBucket("feature/login"), { id: "feature", rest: "login" });
  assert.equal(gitflowBucket("hotfix/fix").id, "hotfix");
  assert.equal(gitflowBucket("cotizador-web").id, "otras");
});

test("se pueden asignar ramas sueltas y prefijos propios", () => {
  const rules = normalizeGitflow({
    enabled: true,
    production: "trunk",
    develop: "integration",
    feature: "cotizador-web, wip/",
  });
  assert.equal(gitflowBucket("trunk", rules).id, "production");
  assert.equal(gitflowBucket("main", rules).id, "otras");
  assert.equal(gitflowBucket("integration", rules).id, "develop");
  assert.equal(gitflowBucket("cotizador-web", rules).id, "feature");
  assert.deepEqual(gitflowBucket("wip/login", rules), { id: "feature", rest: "login" });
});

test("parseGitflowTokens parte por coma y limpia huecos", () => {
  assert.deepEqual(parseGitflowTokens("main, master,  "), ["main", "master"]);
  assert.deepEqual(parseGitflowTokens(""), []);
});

test("el primer tipo gana si una rama está en dos sitios", () => {
  const rules = normalizeGitflow({ enabled: true, production: "main", feature: "main" });
  assert.equal(gitflowBucket("main", rules).id, "production");
});

test("gitflowTokenOk acepta nombres y prefijos, no basura", () => {
  assert.equal(gitflowTokenOk("cotizador-web"), true);
  assert.equal(gitflowTokenOk("wip/"), true);
  assert.equal(gitflowTokenOk("feature/login"), true);
  assert.equal(gitflowTokenOk("*"), false);
  assert.equal(gitflowTokenOk("-bad"), false);
  assert.equal(gitflowTokenOk("foo..bar"), false);
  assert.equal(gitflowTokenOk("rama mala"), false);
  assert.equal(gitflowTokenOk("main\nmaster"), false);
});

test("gitflowCreateKinds solo lista tipos con prefijo", () => {
  const kinds = gitflowCreateKinds(defaultGitflow());
  assert.deepEqual(
    kinds.map((kind) => [kind.id, kind.prefix]),
    [
      ["feature", "feature/"],
      ["release", "release/"],
      ["hotfix", "hotfix/"],
      ["support", "support/"],
    ],
  );
  const custom = gitflowCreateKinds({ feature: "wip/, feat/", develop: "dev/" });
  assert.deepEqual(
    custom.map((kind) => [kind.id, kind.prefix]),
    [
      ["develop", "dev/"],
      ["feature", "wip/"],
      ["release", "release/"],
      ["hotfix", "hotfix/"],
      ["support", "support/"],
    ],
  );
});

test("assertGitflow rechaza un token inválido y deja pasar el resto", () => {
  assert.throws(() => assertGitflow({ production: "main\nmaster" }), /no es válido/);
  assert.equal(assertGitflow({ enabled: true, production: "trunk" }).production, "trunk");
});
