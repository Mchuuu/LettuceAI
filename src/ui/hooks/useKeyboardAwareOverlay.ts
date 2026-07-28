import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import {
  ANDROID_WINDOW_INSETS_EVENT,
  type AndroidWindowInsetsSnapshot,
} from "../../core/utils/androidWindowInsets";
import { getPlatform } from "../../core/utils/platform";

const EDITABLE_CONTROL_SELECTOR = [
  "input:not([type='checkbox']):not([type='radio']):not([type='range'])",
  "textarea",
  "select",
  "[contenteditable='true']",
].join(",");

interface KeyboardAwareOverlayOptions {
  enabled: boolean;
  containerRef: RefObject<HTMLElement | null>;
}

interface KeyboardAwareOverlayResult {
  keyboardInset: string;
}

function isEditableControl(element: Element): element is HTMLElement {
  return element instanceof HTMLElement && element.matches(EDITABLE_CONTROL_SELECTOR);
}

function isAndroidPlatform() {
  try {
    return getPlatform().os === "android";
  } catch {
    return typeof navigator !== "undefined" && /Android/i.test(navigator.userAgent);
  }
}

export function useKeyboardAwareOverlay({
  enabled,
  containerRef,
}: KeyboardAwareOverlayOptions): KeyboardAwareOverlayResult {
  const isAndroid = useMemo(isAndroidPlatform, []);
  const [fallbackInset, setFallbackInset] = useState(0);
  const revealFrameRef = useRef<number | null>(null);

  const revealFocusedControl = useCallback(() => {
    if (typeof document === "undefined") return;

    const container = containerRef.current;
    const activeElement = document.activeElement;
    if (!container || !activeElement || !container.contains(activeElement)) return;
    if (!isEditableControl(activeElement)) return;

    if (revealFrameRef.current !== null) {
      cancelAnimationFrame(revealFrameRef.current);
    }
    revealFrameRef.current = requestAnimationFrame(() => {
      revealFrameRef.current = null;
      activeElement.scrollIntoView({
        block: "nearest",
        inline: "nearest",
        behavior: "auto",
      });
    });
  }, [containerRef]);

  useEffect(() => {
    if (!enabled || !isAndroid) return;

    const handleWindowInsets = (event: Event) => {
      const detail = (event as CustomEvent<AndroidWindowInsetsSnapshot>).detail;
      if (detail?.source !== "animation_end") return;
      if (typeof detail.imeBottomPx !== "number" || detail.imeBottomPx <= 0) return;
      revealFocusedControl();
    };

    const handleFocusIn = () => {
      const imeBottomPx = window.__lettuceWindowInsets?.imeBottomPx;
      if (typeof imeBottomPx === "number" && imeBottomPx > 0) {
        revealFocusedControl();
      }
    };

    window.addEventListener(ANDROID_WINDOW_INSETS_EVENT, handleWindowInsets);
    containerRef.current?.addEventListener("focusin", handleFocusIn);
    const container = containerRef.current;

    return () => {
      window.removeEventListener(ANDROID_WINDOW_INSETS_EVENT, handleWindowInsets);
      container?.removeEventListener("focusin", handleFocusIn);
    };
  }, [containerRef, enabled, isAndroid, revealFocusedControl]);

  useEffect(() => {
    const visualViewport =
      typeof window !== "undefined" ? window.visualViewport : null;
    if (
      !enabled ||
      isAndroid ||
      !visualViewport
    ) {
      setFallbackInset(0);
      return;
    }

    const updateKeyboardInset = () => {
      const baseHeight = window.innerHeight;
      const viewportHeight = visualViewport.height;
      const nextInset = Math.max(0, baseHeight - viewportHeight - visualViewport.offsetTop);
      const roundedInset = nextInset > 0 ? Math.round(nextInset) : 0;
      setFallbackInset(roundedInset);
      if (roundedInset > 0) {
        revealFocusedControl();
      }
    };

    updateKeyboardInset();
    visualViewport.addEventListener("resize", updateKeyboardInset);
    visualViewport.addEventListener("scroll", updateKeyboardInset);
    window.addEventListener("orientationchange", updateKeyboardInset);

    return () => {
      visualViewport.removeEventListener("resize", updateKeyboardInset);
      visualViewport.removeEventListener("scroll", updateKeyboardInset);
      window.removeEventListener("orientationchange", updateKeyboardInset);
      setFallbackInset(0);
    };
  }, [enabled, isAndroid, revealFocusedControl]);

  useEffect(
    () => () => {
      if (revealFrameRef.current !== null) {
        cancelAnimationFrame(revealFrameRef.current);
      }
    },
    [],
  );

  return {
    keyboardInset: isAndroid
      ? "var(--lettuce-keyboard-inset, 0px)"
      : `${fallbackInset}px`,
  };
}
