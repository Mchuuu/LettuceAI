import { useLayoutEffect, useRef, useState, type RefObject } from "react";

import {
  ANDROID_WINDOW_INSETS_EVENT,
  type AndroidWindowInsetsSnapshot,
} from "../../core/utils/androidWindowInsets";

interface AndroidImeMotionOptions {
  enabled: boolean;
  translateRefs: readonly RefObject<HTMLElement | null>[];
}

function readCssPixels(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return 0;
  return Math.max(0, value) / (window.devicePixelRatio || 1);
}

function applyTranslation(
  translateRefs: readonly RefObject<HTMLElement | null>[],
  insetCssPixels: number,
) {
  const transform = `translate3d(0, ${-insetCssPixels}px, 0)`;
  for (const targetRef of translateRefs) {
    if (targetRef.current) targetRef.current.style.transform = transform;
  }
}

function syncRootKeyboardInset(snapshot?: AndroidWindowInsetsSnapshot) {
  const insetCssPixels = readCssPixels(snapshot?.imeBottomPx);
  document.documentElement.style.setProperty("--lettuce-keyboard-inset", `${insetCssPixels}px`);
}

/**
 * Moves a small set of compositor layers directly during Android's IME animation.
 * This keeps per-frame keyboard updates from invalidating the full document tree.
 */
export function useAndroidImeMotion({ enabled, translateRefs }: AndroidImeMotionOptions): number {
  const [statusBarInset, setStatusBarInset] = useState(0);
  const lastStatusBarInsetRef = useRef(0);

  useLayoutEffect(() => {
    if (!enabled) return;

    const applyWindowInsets = (snapshot: AndroidWindowInsetsSnapshot = {}) => {
      applyTranslation(translateRefs, readCssPixels(snapshot.imeBottomPx));

      const nextStatusBarInset = Math.round(readCssPixels(snapshot.statusBarTopPx));
      if (nextStatusBarInset !== lastStatusBarInsetRef.current) {
        lastStatusBarInsetRef.current = nextStatusBarInset;
        setStatusBarInset(nextStatusBarInset);
      }
    };

    const handleWindowInsets = (event: Event) => {
      applyWindowInsets((event as CustomEvent<AndroidWindowInsetsSnapshot>).detail ?? {});
    };

    window.__lettuceDirectImeMotion = true;
    window.addEventListener(ANDROID_WINDOW_INSETS_EVENT, handleWindowInsets);
    applyWindowInsets(window.__lettuceWindowInsets);

    return () => {
      window.removeEventListener(ANDROID_WINDOW_INSETS_EVENT, handleWindowInsets);
      window.__lettuceDirectImeMotion = false;
      syncRootKeyboardInset(window.__lettuceWindowInsets);
      applyTranslation(translateRefs, 0);
    };
  }, [enabled, translateRefs]);

  return enabled ? statusBarInset : 0;
}
