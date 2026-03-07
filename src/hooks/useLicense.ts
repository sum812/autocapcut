import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export type LicenseStatusKind =
  | "Valid"
  | "GracePeriod"
  | "Expired"
  | "NotActivated"
  | "Invalid";

export interface LicenseStatus {
  status: LicenseStatusKind;
  // Valid
  plan?: string;
  license_expires_at?: string | null;
  max_projects?: number | null;
  features?: string[];
  days_until_token_expire?: number;
  // GracePeriod
  expires_in_hours?: number;
  // Expired
  expired_at?: string;
  // Invalid
  reason?: string;
}

export function useLicense() {
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus>({ status: "NotActivated" });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const status = await invoke<LicenseStatus>("get_license_status");
      setLicenseStatus(status);
    } catch {
      // Silently ignore — NotActivated is default
    }
  }, []);

  const activate = useCallback(async (key: string) => {
    setLoading(true);
    setError(null);
    try {
      const status = await invoke<LicenseStatus>("activate_license", { key });
      setLicenseStatus(status);
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  const deactivate = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("deactivate_license");
      setLicenseStatus({ status: "NotActivated" });
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const status = await invoke<LicenseStatus>("refresh_license");
      setLicenseStatus(status);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Load status on mount
  useEffect(() => {
    fetchStatus();
  }, []);

  const isPro = licenseStatus.status === "Valid" || licenseStatus.status === "GracePeriod";
  const projectLimit = isPro ? (licenseStatus.max_projects ?? null) : 3;

  return {
    licenseStatus,
    loading,
    error,
    isPro,
    projectLimit,
    activate,
    deactivate,
    refresh,
    clearError: () => setError(null),
  };
}
