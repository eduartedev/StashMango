export const services = [
  {
    id: "github",
    label: "GitHub",
    host: "github.com",
    hint: "Token con acceso a repo",
  },
  {
    id: "gitlab",
    label: "GitLab",
    host: "gitlab.com",
    hint: "Token con alcance api",
  },
  {
    id: "bitbucket",
    label: "Bitbucket",
    host: "bitbucket.org",
    hint: "Access token HTTP",
  },
];

export function emptyServiceTokens() {
  return Object.fromEntries(services.map((item) => [item.host, ""]));
}

export function emptyServiceSaved() {
  return Object.fromEntries(services.map((item) => [item.host, false]));
}

export function tokenPage(host) {
  if (host === "github.com") {
    return "https://github.com/settings/tokens/new?scopes=repo,read:org&description=StashMango";
  }
  if (host === "gitlab.com") {
    return "https://gitlab.com/-/user_settings/personal_access_tokens?name=StashMango&scopes=api";
  }
  if (host === "bitbucket.org") {
    return "https://bitbucket.org/account/settings/access-tokens/";
  }
  return "";
}

export function gitHadTokenNotice(label) {
  return `Git tenía un token para ${label}. Guárdalo si quieres usarlo aquí.`;
}
