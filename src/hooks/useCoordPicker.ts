import { useState, useEffect, useCallback, useRef } from "react";
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

  // Dùng refs để tránh stale closure — listener chỉ đăng ký 1 lần
  const pickingRef = useRef<PickerKey | null>(null);
  const onPickRef = useRef(onPick);

  // Sync refs với state/props mới nhất
  pickingRef.current = picking;
  onPickRef.current = onPick;

  const startPick = useCallback(async (key: PickerKey) => {
    setPicking(key);
    await invoke("start_picking_coords");
  }, []);

  const cancelPick = useCallback(async () => {
    setPicking(null);
    await invoke("cancel_picking_coords");
  }, []);

  // Đăng ký listener 1 lần duy nhất — dùng ref để đọc picking hiện tại
  useEffect(() => {
    let unlistenPick: (() => void) | undefined;
    let unlistenCancel: (() => void) | undefined;

    listen<Coords>("coordinate-picked", async (event) => {
      const key = pickingRef.current;
      if (key) {
        const [x, y] = event.payload;
        onPickRef.current(key, event.payload);
        setPicking(null);
        try {
          await invoke("capture_ui_template", { coordKey: key, x, y });
        } catch {
          // Lỗi template capture không chặn workflow thủ công
        }
      }
    }).then((u) => (unlistenPick = u));

    listen("coordinate-picked-cancel", () => {
      setPicking(null);
    }).then((u) => (unlistenCancel = u));

    return () => {
      unlistenPick?.();
      unlistenCancel?.();
    };
  }, []); // empty deps — chỉ mount/unmount

  return { picking, startPick, cancelPick };
}
