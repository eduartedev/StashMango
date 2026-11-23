import assert from "node:assert/strict";
import test from "node:test";
import {
  emptyServiceSaved,
  emptyServiceTokens,
  gitHadTokenNotice,
  services,
  tokenPage,
} from "./services.js";

test("los tres forges tienen página de token https fija", () => {
  assert.deepEqual(services.map((item) => item.host), ["github.com", "gitlab.com", "bitbucket.org"]);
  for (const item of services) {
    const url = tokenPage(item.host);
    assert.equal(new URL(url).protocol, "https:");
    assert.equal(new URL(url).hostname, item.host);
  }
  assert.equal(tokenPage("evil.example"), "");
  assert.equal(tokenPage(""), "");
});

test("GitHub pide repo y GitLab pide api", () => {
  assert.equal(tokenPage("github.com").includes("scopes=repo"), true);
  assert.equal(tokenPage("gitlab.com").includes("scopes=api"), true);
  assert.equal(tokenPage("bitbucket.org").startsWith("https://bitbucket.org/"), true);
});

test("el aviso de git no incluye el secreto", () => {
  const notice = gitHadTokenNotice("GitHub");
  assert.equal(notice.includes("GitHub"), true);
  assert.equal(notice.includes("ghp_"), false);
  assert.equal(notice.includes("token ghp"), false);
});

test("cada servicio tiene el id de marca de su logo", () => {
  assert.deepEqual(services.map((item) => item.id), ["github", "gitlab", "bitbucket"]);
});

test("los mapas vacíos cubren solo los hosts conocidos", () => {
  assert.deepEqual(Object.keys(emptyServiceTokens()), ["github.com", "gitlab.com", "bitbucket.org"]);
  assert.deepEqual(emptyServiceSaved()["github.com"], false);
  assert.equal(emptyServiceTokens()["github.com"], "");
});
