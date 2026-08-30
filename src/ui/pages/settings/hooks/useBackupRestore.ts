import { listen } from "@tauri-apps/api/event";
import { useReducer, useCallback, useEffect, useMemo, useRef } from "react";
import {
  storageBridge,
  type BackupEstimate,
  type BackupSelection,
  type BackupSelectionMode,
} from "../../../../core/storage/files";
import { setTooltipSeen } from "../../../../core/storage/appState";

function toErrorMessage(e: unknown, fallback: string): string {
  if (typeof e === "string" && e.trim().length > 0) return e;
  if (e instanceof Error && e.message.trim().length > 0) return e.message;
  return fallback;
}

export interface BackupInfo {
  version: number;
  createdAt: number;
  appVersion: string;
  encrypted: boolean;
  custom?: boolean;
  totalFiles: number;
  imageCount: number;
  avatarCount: number;
  attachmentCount: number;
  path: string;
  filename: string;
}

export interface BackupExportProgress {
  requestId: string;
  stage: "preparing" | "data" | "resources" | "finalizing" | "completed" | "failed";
  progress: number;
  completedItems: number;
  totalItems: number;
  completedBytes: number;
  totalBytes: number;
  currentItem?: string | null;
  error?: string | null;
}

interface CharacterChoice {
  selected: boolean;
  includeAudio: boolean;
  includeImages: boolean;
}

type ModalType = "export" | "import" | "delete" | null;

interface State {
  backups: BackupInfo[];
  loading: boolean;
  exporting: boolean;
  importing: boolean;
  activeModal: ModalType;
  selectedBackup: BackupInfo | null;
  isPickedFile: boolean;
  exportPassword: string;
  confirmPassword: string;
  importPassword: string;
  showExportPassword: boolean;
  showImportPassword: boolean;
  exportSuccess: string | null;
  error: string | null;
  selectionMode: BackupSelectionMode;
  characterChoices: Record<string, CharacterChoice>;
  includeUnassignedAudio: boolean;
  customExpanded: boolean;
  estimate: BackupEstimate | null;
  estimating: boolean;
  exportProgress: BackupExportProgress | null;
}

type Action =
  | { type: "SET_BACKUPS"; payload: BackupInfo[] }
  | { type: "SET_LOADING"; payload: boolean }
  | { type: "SET_EXPORTING"; payload: boolean }
  | { type: "SET_IMPORTING"; payload: boolean }
  | { type: "OPEN_EXPORT_MODAL" }
  | { type: "OPEN_IMPORT_MODAL"; payload: BackupInfo }
  | { type: "OPEN_IMPORT_MODAL_PICKED"; payload: BackupInfo }
  | { type: "OPEN_DELETE_MODAL"; payload: BackupInfo }
  | { type: "CLOSE_MODAL" }
  | { type: "SET_EXPORT_PASSWORD"; payload: string }
  | { type: "SET_CONFIRM_PASSWORD"; payload: string }
  | { type: "SET_IMPORT_PASSWORD"; payload: string }
  | { type: "TOGGLE_SHOW_EXPORT_PASSWORD" }
  | { type: "TOGGLE_SHOW_IMPORT_PASSWORD" }
  | { type: "SET_EXPORT_SUCCESS"; payload: string | null }
  | { type: "SET_ERROR"; payload: string | null }
  | { type: "SET_SELECTION_MODE"; payload: BackupSelectionMode }
  | { type: "SET_ESTIMATING"; payload: boolean }
  | { type: "SET_ESTIMATE"; payload: BackupEstimate }
  | { type: "TOGGLE_CUSTOM_EXPANDED" }
  | { type: "TOGGLE_CHARACTER"; payload: string }
  | { type: "TOGGLE_CHARACTER_RESOURCE"; payload: { id: string; resource: "audio" | "images" } }
  | { type: "SET_ALL_CHARACTERS"; payload: boolean }
  | { type: "TOGGLE_UNASSIGNED_AUDIO" }
  | { type: "SET_EXPORT_PROGRESS"; payload: BackupExportProgress | null }
  | { type: "EXPORT_COMPLETE"; payload: string }
  | { type: "IMPORT_COMPLETE" }
  | { type: "DELETE_COMPLETE" };

const initialState: State = {
  backups: [],
  loading: true,
  exporting: false,
  importing: false,
  activeModal: null,
  selectedBackup: null,
  isPickedFile: false,
  exportPassword: "",
  confirmPassword: "",
  importPassword: "",
  showExportPassword: false,
  showImportPassword: false,
  exportSuccess: null,
  error: null,
  selectionMode: "full",
  characterChoices: {},
  includeUnassignedAudio: true,
  customExpanded: false,
  estimate: null,
  estimating: false,
  exportProgress: null,
};

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_BACKUPS":
      return { ...state, backups: action.payload };
    case "SET_LOADING":
      return { ...state, loading: action.payload };
    case "SET_EXPORTING":
      return { ...state, exporting: action.payload };
    case "SET_IMPORTING":
      return { ...state, importing: action.payload };
    case "OPEN_EXPORT_MODAL":
      return {
        ...state,
        activeModal: "export",
        exportPassword: "",
        confirmPassword: "",
        selectionMode: "full",
        characterChoices: {},
        includeUnassignedAudio: true,
        customExpanded: false,
        estimate: null,
        exportProgress: null,
        error: null,
      };
    case "OPEN_IMPORT_MODAL":
    case "OPEN_IMPORT_MODAL_PICKED":
      return {
        ...state,
        activeModal: "import",
        selectedBackup: action.payload,
        isPickedFile: action.type === "OPEN_IMPORT_MODAL_PICKED",
        importPassword: "",
        error: null,
      };
    case "OPEN_DELETE_MODAL":
      return { ...state, activeModal: "delete", selectedBackup: action.payload, error: null };
    case "CLOSE_MODAL":
      return { ...state, activeModal: null };
    case "SET_EXPORT_PASSWORD":
      return { ...state, exportPassword: action.payload };
    case "SET_CONFIRM_PASSWORD":
      return { ...state, confirmPassword: action.payload };
    case "SET_IMPORT_PASSWORD":
      return { ...state, importPassword: action.payload };
    case "TOGGLE_SHOW_EXPORT_PASSWORD":
      return { ...state, showExportPassword: !state.showExportPassword };
    case "TOGGLE_SHOW_IMPORT_PASSWORD":
      return { ...state, showImportPassword: !state.showImportPassword };
    case "SET_EXPORT_SUCCESS":
      return { ...state, exportSuccess: action.payload };
    case "SET_ERROR":
      return { ...state, error: action.payload };
    case "SET_SELECTION_MODE":
      return {
        ...state,
        selectionMode: action.payload,
        customExpanded: action.payload === "custom" ? true : state.customExpanded,
      };
    case "SET_ESTIMATING":
      return { ...state, estimating: action.payload };
    case "SET_ESTIMATE": {
      const choices = { ...state.characterChoices };
      for (const character of action.payload.characters) {
        choices[character.characterId] ??= {
          selected: true,
          includeAudio: true,
          includeImages: true,
        };
      }
      return { ...state, estimate: action.payload, characterChoices: choices, estimating: false };
    }
    case "TOGGLE_CUSTOM_EXPANDED":
      return { ...state, customExpanded: !state.customExpanded };
    case "TOGGLE_CHARACTER": {
      const current = state.characterChoices[action.payload];
      if (!current) return state;
      return {
        ...state,
        characterChoices: {
          ...state.characterChoices,
          [action.payload]: { ...current, selected: !current.selected },
        },
      };
    }
    case "TOGGLE_CHARACTER_RESOURCE": {
      const { id, resource } = action.payload;
      const current = state.characterChoices[id];
      if (!current) return state;
      const key = resource === "audio" ? "includeAudio" : "includeImages";
      return {
        ...state,
        characterChoices: {
          ...state.characterChoices,
          [id]: { ...current, [key]: !current[key] },
        },
      };
    }
    case "SET_ALL_CHARACTERS": {
      const choices = Object.fromEntries(
        Object.entries(state.characterChoices).map(([id, choice]) => [
          id,
          { ...choice, selected: action.payload },
        ]),
      );
      return { ...state, characterChoices: choices };
    }
    case "TOGGLE_UNASSIGNED_AUDIO":
      return { ...state, includeUnassignedAudio: !state.includeUnassignedAudio };
    case "SET_EXPORT_PROGRESS":
      return { ...state, exportProgress: action.payload };
    case "EXPORT_COMPLETE":
      return {
        ...state,
        exporting: false,
        activeModal: null,
        exportPassword: "",
        confirmPassword: "",
        exportSuccess: action.payload,
      };
    case "IMPORT_COMPLETE":
      return {
        ...state,
        importing: false,
        activeModal: null,
        importPassword: "",
        selectedBackup: null,
        isPickedFile: false,
      };
    case "DELETE_COMPLETE":
      return { ...state, activeModal: null, selectedBackup: null, isPickedFile: false };
    default:
      return state;
  }
}

function createSelection(state: State): BackupSelection {
  return {
    mode: state.selectionMode,
    characters:
      state.selectionMode === "custom"
        ? Object.entries(state.characterChoices)
            .filter(([, choice]) => choice.selected)
            .map(([characterId, choice]) => ({
              characterId,
              includeAudio: choice.includeAudio,
              includeImages: choice.includeImages,
            }))
        : [],
    includeUnassignedAudio: state.includeUnassignedAudio,
  };
}

function createRequestId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `backup-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function useBackupRestore() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const estimateSequence = useRef(0);

  const loadBackups = useCallback(async () => {
    try {
      dispatch({ type: "SET_LOADING", payload: true });
      dispatch({ type: "SET_BACKUPS", payload: await storageBridge.backupList() });
    } catch (e) {
      console.error("Failed to load backups:", e);
    } finally {
      dispatch({ type: "SET_LOADING", payload: false });
    }
  }, []);

  useEffect(() => {
    void loadBackups();
  }, [loadBackups]);

  const selectionFingerprint = useMemo(
    () =>
      state.selectionMode === "full"
        ? "full"
        : JSON.stringify({
            choices: state.characterChoices,
            includeUnassignedAudio: state.includeUnassignedAudio,
          }),
    [state.selectionMode, state.characterChoices, state.includeUnassignedAudio],
  );

  useEffect(() => {
    if (state.activeModal !== "export" || state.exporting) return;
    const sequence = ++estimateSequence.current;
    dispatch({ type: "SET_ESTIMATING", payload: true });
    const timer = window.setTimeout(async () => {
      try {
        const estimate = await storageBridge.backupEstimate(createSelection(state));
        if (sequence === estimateSequence.current) {
          dispatch({ type: "SET_ESTIMATE", payload: estimate });
        }
      } catch (error) {
        if (sequence === estimateSequence.current) {
          dispatch({ type: "SET_ESTIMATING", payload: false });
          dispatch({ type: "SET_ERROR", payload: toErrorMessage(error, "Estimate failed") });
        }
      }
    }, state.estimate ? 250 : 0);
    return () => window.clearTimeout(timer);
    // The fingerprint intentionally captures only fields that alter the backup plan.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.activeModal, state.exporting, selectionFingerprint]);

  const handleExport = useCallback(async () => {
    if (state.exportPassword.length < 6) {
      dispatch({ type: "SET_ERROR", payload: "Password must be at least 6 characters" });
      return;
    }
    if (state.exportPassword !== state.confirmPassword) {
      dispatch({ type: "SET_ERROR", payload: "Passwords do not match" });
      return;
    }
    const selection = createSelection(state);
    if (selection.mode === "custom" && selection.characters.length === 0) {
      dispatch({ type: "SET_ERROR", payload: "Select at least one character" });
      return;
    }

    const requestId = createRequestId();
    let unlisten: (() => void) | undefined;
    try {
      dispatch({ type: "SET_ERROR", payload: null });
      dispatch({ type: "SET_EXPORTING", payload: true });
      dispatch({
        type: "SET_EXPORT_PROGRESS",
        payload: {
          requestId,
          stage: "preparing",
          progress: 0,
          completedItems: 0,
          totalItems: 0,
          completedBytes: 0,
          totalBytes: state.estimate?.totalBytes ?? 0,
        },
      });
      unlisten = await listen<BackupExportProgress>("backup-export-progress", ({ payload }) => {
        if (payload.requestId === requestId) {
          dispatch({ type: "SET_EXPORT_PROGRESS", payload });
        }
      });
      const path = await storageBridge.backupExport(state.exportPassword, selection, requestId);
      dispatch({ type: "EXPORT_COMPLETE", payload: path });
      await loadBackups();
    } catch (e) {
      const message = toErrorMessage(e, "Export failed");
      dispatch({ type: "SET_ERROR", payload: message });
      dispatch({
        type: "SET_EXPORT_PROGRESS",
        payload: {
          requestId,
          stage: "failed",
          progress: 0,
          completedItems: 0,
          totalItems: 0,
          completedBytes: 0,
          totalBytes: state.estimate?.totalBytes ?? 0,
          error: message,
        },
      });
      dispatch({ type: "SET_EXPORTING", payload: false });
    } finally {
      unlisten?.();
    }
  }, [state, loadBackups]);

  const handleImport = useCallback(async () => {
    const { selectedBackup, importPassword } = state;
    if (!selectedBackup) return;
    if (selectedBackup.encrypted && importPassword.length < 1) {
      dispatch({ type: "SET_ERROR", payload: "Password is required for this backup" });
      return;
    }
    try {
      dispatch({ type: "SET_ERROR", payload: null });
      if (
        selectedBackup.encrypted &&
        !(await storageBridge.backupVerifyPassword(selectedBackup.path, importPassword))
      ) {
        dispatch({ type: "SET_ERROR", payload: "Incorrect password" });
        return;
      }
      dispatch({ type: "SET_IMPORTING", payload: true });
      await storageBridge.backupImport(
        selectedBackup.path,
        selectedBackup.encrypted ? importPassword : undefined,
      );
      dispatch({ type: "IMPORT_COMPLETE" });
      await Promise.all([
        setTooltipSeen("app_tour_v1"),
        setTooltipSeen("chat_detail_tour_v1"),
        setTooltipSeen("post_first_message_tour_v1"),
      ]);
      return { success: true };
    } catch (e) {
      dispatch({ type: "SET_ERROR", payload: toErrorMessage(e, "Import failed") });
      dispatch({ type: "SET_IMPORTING", payload: false });
      return { error: true };
    }
  }, [state]);

  const handleDelete = useCallback(async () => {
    if (!state.selectedBackup) return;
    try {
      dispatch({ type: "SET_ERROR", payload: null });
      await storageBridge.backupDelete(state.selectedBackup.path);
      dispatch({ type: "DELETE_COMPLETE" });
      await loadBackups();
    } catch (e) {
      dispatch({ type: "SET_ERROR", payload: toErrorMessage(e, "Delete failed") });
    }
  }, [state.selectedBackup, loadBackups]);

  const handleBrowseForBackup = useCallback(async () => {
    try {
      const result = await storageBridge.backupPickFile();
      if (!result) return;
      const info = await storageBridge.backupGetInfo(result.path);
      dispatch({
        type: "OPEN_IMPORT_MODAL",
        payload: { ...info, path: result.path, filename: result.filename },
      });
    } catch (e) {
      dispatch({ type: "SET_ERROR", payload: toErrorMessage(e, "Failed to open file") });
    }
  }, []);

  const actions = {
    openExportModal: () => dispatch({ type: "OPEN_EXPORT_MODAL" } as Action),
    openImportModal: (backup: BackupInfo) =>
      dispatch({ type: "OPEN_IMPORT_MODAL", payload: backup }),
    openDeleteModal: (backup: BackupInfo) =>
      dispatch({ type: "OPEN_DELETE_MODAL", payload: backup }),
    closeModal: () => dispatch({ type: "CLOSE_MODAL" } as Action),
    setExportPassword: (value: string) =>
      dispatch({ type: "SET_EXPORT_PASSWORD", payload: value }),
    setConfirmPassword: (value: string) =>
      dispatch({ type: "SET_CONFIRM_PASSWORD", payload: value }),
    setImportPassword: (value: string) =>
      dispatch({ type: "SET_IMPORT_PASSWORD", payload: value }),
    toggleShowExportPassword: () => dispatch({ type: "TOGGLE_SHOW_EXPORT_PASSWORD" } as Action),
    toggleShowImportPassword: () => dispatch({ type: "TOGGLE_SHOW_IMPORT_PASSWORD" } as Action),
    clearExportSuccess: () => dispatch({ type: "SET_EXPORT_SUCCESS", payload: null }),
    setSelectionMode: (mode: BackupSelectionMode) =>
      dispatch({ type: "SET_SELECTION_MODE", payload: mode }),
    toggleCustomExpanded: () => dispatch({ type: "TOGGLE_CUSTOM_EXPANDED" } as Action),
    toggleCharacter: (id: string) => dispatch({ type: "TOGGLE_CHARACTER", payload: id }),
    toggleCharacterResource: (id: string, resource: "audio" | "images") =>
      dispatch({ type: "TOGGLE_CHARACTER_RESOURCE", payload: { id, resource } }),
    setAllCharacters: (selected: boolean) =>
      dispatch({ type: "SET_ALL_CHARACTERS", payload: selected }),
    toggleUnassignedAudio: () => dispatch({ type: "TOGGLE_UNASSIGNED_AUDIO" } as Action),
    handleExport,
    handleImport,
    handleDelete,
    handleBrowseForBackup,
  };

  return { state, actions };
}
