import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from "react";
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
  strategy?: "layout" | "compositor";
  syncLayoutInsetDuringAnimation?: boolean;
}

interface KeyboardAwareOverlayResult {
  keyboardInset: string;
  keyboardTransform?: string;
}

const OVERLAY_KEYBOARD_OFFSET = "--lettuce-overlay-keyboard-offset";
const OVERLAY_KEYBOARD_LAYOUT_INSET = "--lettuce-overlay-keyboard-layout-inset";

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
  strategy = "layout",
  syncLayoutInsetDuringAnimation = false,
}: KeyboardAwareOverlayOptions): KeyboardAwareOverlayResult {
  const isAndroid = useMemo(isAndroidPlatform, []);
  const usesCompositorMotion = isAndroid && strategy === "compositor";
  const [fallbackInset, setFallbackInset] = useState(0);
  const revealFrameRef = useRef<number | null>(null);

  useLayoutEffect(() => {
    if (!enabled || !usesCompositorMotion) return;

    const applyInsets = (snapshot: AndroidWindowInsetsSnapshot = {}) => {
      const rawInset = snapshot.imeBottomPx;
      const inset =
        typeof rawInset === "number" && Number.isFinite(rawInset)
          ? Math.max(0, rawInset) / (window.devicePixelRatio || 1)
          : 0;
      const insetValue = `${inset}px`;
      const container = containerRef.current;
      if (!container) return;

      container.style.setProperty(OVERLAY_KEYBOARD_OFFSET, insetValue);
      if (syncLayoutInsetDuringAnimation || snapshot.source !== "animation_progress") {
        container.style.setProperty(OVERLAY_KEYBOARD_LAYOUT_INSET, insetValue);
      }
    };

    const handleWindowInsets = (event: Event) => {
      applyInsets((event as CustomEvent<AndroidWindowInsetsSnapshot>).detail ?? {});
    };

    window.addEventListener(ANDROID_WINDOW_INSETS_EVENT, handleWindowInsets);
    applyInsets(window.__lettuceWindowInsets);

    return () => {
      window.removeEventListener(ANDROID_WINDOW_INSETS_EVENT, handleWindowInsets);
      containerRef.current?.style.removeProperty(OVERLAY_KEYBOARD_OFFSET);
      containerRef.current?.style.removeProperty(OVERLAY_KEYBOARD_LAYOUT_INSET);
    };
  }, [containerRef, enabled, syncLayoutInsetDuringAnimation, usesCompositorMotion]);

  const revealFocusedControl = useCallback(() => {
    if (typeof document === "undefined") return;

    const container = containerRef.current;
    const activeElement = document.activeElement;
    if (!container || !activeElement || !container.contains(activeElement)) return;
    if (!isEditableControl(activeElement)) return;
    if (activeElement.dataset.keyboardReveal === "self") return;

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
    keyboardInset: usesCompositorMotion
      ? `var(${OVERLAY_KEYBOARD_LAYOUT_INSET}, 0px)`
      : isAndroid
        ? "var(--lettuce-keyboard-inset, 0px)"
        : `${fallbackInset}px`,
    keyboardTransform: usesCompositorMotion
      ? `translate3d(0, calc(var(${OVERLAY_KEYBOARD_OFFSET}, 0px) * -1), 0)`
      : undefined,
  };
}
