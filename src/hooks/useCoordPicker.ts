import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Coords = [number, number];
type PickerKey =
  | "first_project_coords"
  | "export_box_coords"
  | "search_button_coords";

export function useCoordPicker(
  onPick: (key: PickerKey, coords: Coords) => void
) {
  const [picking, setPicking] = useState<PickerKey | null>(null);

  const startPick = useCallback(async (key: PickerKey) => {
    setPicking(key);
    await invoke("start_picking_coords");
  }, []);

  const cancelPick = useCallback(async () => {
    setPicking(null);
    await invoke("cancel_picking_coords");
  }, []);

  useEffect(() => {
    let unlistenPick: (() => void) | undefined;
    let unlistenCancel: (() => void) | undefined;

    listen<Coords>("coordinate-picked", (event) => {
      if (picking) {
        onPick(picking, event.payload);
        setPicking(null);
      }
    }).then((u) => (unlistenPick = u));

    listen("coordinate-picked-cancel", () => {
      setPicking(null);
    }).then((u) => (unlistenCancel = u));

    return () => {
      unlistenPick?.();
      unlistenCancel?.();
    };
  }, [picking, onPick]);

  return { picking, startPick, cancelPick };
}
