import assert from "node:assert/strict";
import test from "node:test";
import { defaultRepoLive, gitHooksDoc, gitHookNames, presentHooks } from "./gitHooks.js";

test("solo lista hooks conocidos, sin samples ni duplicados", () => {
  assert.ok(gitHookNames.includes("pre-commit"));
  assert.ok(gitHookNames.includes("pre-push"));
  assert.deepEqual(presentHooks(["pre-commit", "pre-commit.sample", "rm", "pre-push", "pre-commit"]), [
    "pre-commit",
    "pre-push",
  ]);
  assert.deepEqual(presentHooks(undefined), []);
});

test("la ayuda apunta a git-scm, no a un cliente ajeno", () => {
  assert.equal(gitHooksDoc.startsWith("https://git-scm.com/docs/githooks"), true);
  assert.equal(defaultRepoLive().skipHooks, false);
});
