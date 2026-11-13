export const settingsNav = [
  { id: "perfiles", label: "Perfiles", icon: "profile" },
  { id: "servicios", label: "Servicios", icon: "token" },
  { id: "ssh", label: "Frase SSH", icon: "key" },
  { id: "git", label: "Git", icon: "branch" },
];

export function settingsSectionOr(id) {
  return settingsNav.some((item) => item.id === id) ? id : "perfiles";
}
