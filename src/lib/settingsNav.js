export const settingsNav = [
  { id: "perfiles", label: "Perfiles", icon: "profile" },
  { id: "servicios", label: "Servicios", icon: "token" },
  { id: "ssh", label: "Frase SSH", icon: "key" },
  { id: "plantillas", label: "Plantillas", icon: "template" },
  { id: "git", label: "Git", icon: "branch" },
  { id: "columnas", label: "Columnas", icon: "sliders" },
  { id: "apariencia", label: "Apariencia", icon: "palette" },
];

export function settingsSectionOr(id) {
  return settingsNav.some((item) => item.id === id) ? id : "perfiles";
}
