import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppConfig } from "../utils/SettingsContext";

export interface ValidationError {
  field: string;
  message: string;
}

export interface ValidationResult {
  ok: boolean;
  errors: ValidationError[];
}

export function useValidation() {
  const [errors, setErrors] = useState<ValidationError[]>([]);

  const validate = useCallback(
    async (config: AppConfig, projectNames: string[]): Promise<boolean> => {
      try {
        const result = await invoke<ValidationResult>("validate_config", {
          projectFolder: config.project_folder,
          exportFolder: config.export_folder,
          firstProjectCoords: config.first_project_coords,
          exportBoxCoords: config.export_box_coords,
          projectNames,
        });
        setErrors(result.errors);
        return result.ok;
      } catch (e) {
        setErrors([{ field: "system", message: String(e) }]);
        return false;
      }
    },
    []
  );

  const clearErrors = useCallback(() => setErrors([]), []);

  return { errors, validate, clearErrors };
}
