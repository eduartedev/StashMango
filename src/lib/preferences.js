export const defaultPreferences = {
  // UI General
  showToolbarLabels: true,
  showAuthorInitials: false,
  showGhostBranch: true,
  highlightBranchRows: true,
  autoGroupWorktrees: true,
  showSpellCheck: true,

  // Grafo
  graphBranchVisibility: "all", // all, local, remote, none
  showBranchesInGraph: true,
  showTagsInGraph: true,
  showAuthorInGraph: true,
  showDateInGraph: true,
  showMessageInGraph: true,
  showDescriptionInGraph: "always", // always, hover, never
  showShaInGraph: false,
  showTreeInGraph: false,

  // Colores
  graphColorIndex: 0,
  defaultGroupColor: "gray",
};

export function normalizePreferences(prefs = {}) {
  return { ...defaultPreferences, ...prefs };
}

export const branchVisibilityOptions = [
  { id: "all", label: "Todas" },
  { id: "local", label: "Solo locales" },
  { id: "remote", label: "Solo remotas" },
  { id: "none", label: "Ninguna" },
];

export const descriptionVisibilityOptions = [
  { id: "always", label: "Siempre" },
  { id: "hover", label: "Al hover" },
  { id: "never", label: "Nunca" },
];

export const groupColorOptions = [
  { id: "gray", label: "Gris" },
  { id: "mango", label: "Mango" },
  { id: "teal", label: "Teal" },
  { id: "rose", label: "Rosa" },
];
