export const gitHooksDoc = "https://git-scm.com/docs/githooks";

export const gitHookNames = [
  "applypatch-msg",
  "pre-applypatch",
  "post-applypatch",
  "pre-commit",
  "pre-merge-commit",
  "prepare-commit-msg",
  "commit-msg",
  "post-commit",
  "pre-rebase",
  "post-checkout",
  "post-merge",
  "pre-push",
  "pre-auto-gc",
  "post-rewrite",
  "sendemail-validate",
  "fsmonitor-watchman",
];

export function presentHooks(names) {
  const allow = new Set(gitHookNames);
  return [...new Set((Array.isArray(names) ? names : []).map((item) => String(item || "").trim()).filter((item) => allow.has(item)))].sort();
}

export function defaultRepoLive() {
  return {
    hooksDir: "",
    skipHooks: false,
    hooks: [],
  };
}
