import { invoke } from "@tauri-apps/api/core";

export function call(command, args = {}) {
  return invoke(command, args);
}
