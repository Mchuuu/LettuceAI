import { useCallback, useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { Mic } from "lucide-react";
import { cn } from "../design-tokens";

const CANCEL_DISTANCE_PX = 56;

export function VoiceComposerControl({
  recording,
  transcribing,
  disabled,
  elapsedMs,
  analyser,
  idleLabel,
  recordingLabel,
  cancelLabel,
  transcribingLabel,
  onStart,
  onRelease,
  onCancel,
}: {
  recording: boolean;
  transcribing: boolean;
  disabled?: boolean;
  elapsedMs: number;
  analyser: AnalyserNode | null;
  idleLabel: string;
  recordingLabel: string;
  cancelLabel: string;
  transcribingLabel: string;
  onStart: () => Promise<void> | void;
  onRelease: () => Promise<void> | void;
  onCancel: () => Promise<void> | void;
}) {
  const activePointerRef = useRef<number | null>(null);
  const startYRef = useRef(0);
  const startPromiseRef = useRef<Promise<void> | null>(null);
  const cancelPendingRef = useRef(false);
  const finishingRef = useRef(false);
  const releaseListenersCleanupRef = useRef<(() => void) | null>(null);
  const [pressed, setPressed] = useState(false);
  const [cancelPending, setCancelPending] = useState(false);

  const resetGesture = useCallback(() => {
    activePointerRef.current = null;
    startPromiseRef.current = null;
    finishingRef.current = false;
    releaseListenersCleanupRef.current?.();
    releaseListenersCleanupRef.current = null;
    setPressed(false);
    setCancelPending(false);
    cancelPendingRef.current = false;
  }, []);

  const finishGesture = useCallback(
    async (cancel: boolean) => {
      if (activePointerRef.current === null || finishingRef.current) return;
      finishingRef.current = true;
      console.info("[ASR hold-to-send] finish", { cancel });
      const startPromise = startPromiseRef.current;
      activePointerRef.current = null;
      setPressed(false);
      try {
        await startPromise;
        console.info("[ASR hold-to-send] recorder ready; applying release");
        if (cancel) await onCancel();
        else await onRelease();
      } finally {
        resetGesture();
      }
    },
    [onCancel, onRelease, resetGesture],
  );

  useEffect(() => {
    if (!disabled) return;
    resetGesture();
  }, [disabled, resetGesture]);

  useEffect(() => {
    return () => resetGesture();
  }, [resetGesture]);

  const armReleaseListeners = useCallback(() => {
    releaseListenersCleanupRef.current?.();
    const release = () => void finishGesture(cancelPendingRef.current);
    const cancel = () => void finishGesture(true);
    document.addEventListener("pointerup", release, true);
    document.addEventListener("mouseup", release, true);
    document.addEventListener("touchend", release, true);
    document.addEventListener("pointercancel", cancel, true);
    document.addEventListener("touchcancel", cancel, true);
    window.addEventListener("blur", cancel, true);
    releaseListenersCleanupRef.current = () => {
      document.removeEventListener("pointerup", release, true);
      document.removeEventListener("mouseup", release, true);
      document.removeEventListener("touchend", release, true);
      document.removeEventListener("pointercancel", cancel, true);
      document.removeEventListener("touchcancel", cancel, true);
      window.removeEventListener("blur", cancel, true);
    };
  }, [finishGesture]);

  return (
    <motion.button
      type="button"
      disabled={disabled || transcribing}
      animate={{ scale: pressed && !cancelPending ? 0.985 : 1 }}
      transition={{ duration: 0.12 }}
      className={cn(
        "relative flex h-10.75 min-w-0 flex-1 select-none items-center justify-center overflow-hidden px-3",
        "border-0 bg-transparent text-sm font-semibold text-fg/75",
        "touch-none transition-colors disabled:cursor-not-allowed disabled:opacity-45",
        (pressed || recording) && !cancelPending && "text-fg",
        cancelPending && "text-danger",
      )}
      onContextMenu={(event) => event.preventDefault()}
      onPointerDown={(event) => {
        if (disabled || transcribing || activePointerRef.current !== null || event.button !== 0)
          return;
        event.preventDefault();
        activePointerRef.current = event.pointerId;
        startYRef.current = event.clientY;
        event.currentTarget.setPointerCapture(event.pointerId);
        setPressed(true);
        setCancelPending(false);
        cancelPendingRef.current = false;
        console.info("[ASR hold-to-send] press", { pointerId: event.pointerId });
        startPromiseRef.current = Promise.resolve(onStart());
        armReleaseListeners();
      }}
      onPointerMove={(event) => {
        if (activePointerRef.current !== event.pointerId) return;
        const nextCancelPending = startYRef.current - event.clientY >= CANCEL_DISTANCE_PX;
        cancelPendingRef.current = nextCancelPending;
        setCancelPending(nextCancelPending);
      }}
      onPointerUp={(event) => {
        if (activePointerRef.current === null) return;
        event.preventDefault();
        void finishGesture(cancelPendingRef.current);
      }}
      onPointerCancel={() => {
        if (activePointerRef.current === null) return;
        void finishGesture(true);
      }}
      onTouchEnd={() => {
        if (activePointerRef.current === null) return;
        void finishGesture(cancelPendingRef.current);
      }}
      onMouseUp={() => {
        if (activePointerRef.current === null) return;
        void finishGesture(cancelPendingRef.current);
      }}
      onLostPointerCapture={(event) => {
        if (activePointerRef.current !== event.pointerId) return;
        void finishGesture(cancelPendingRef.current);
      }}
      aria-label={idleLabel}
    >
      {(recording || pressed) && !cancelPending && !transcribing && (
        <motion.span
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-8 bottom-0 h-px bg-accent/70 blur-[2px]"
          animate={{ opacity: [0.2, 0.9, 0.2], scaleX: [0.65, 1, 0.65] }}
          transition={{ duration: 1.25, repeat: Infinity, ease: "easeInOut" }}
        />
      )}
      <span className="relative flex min-w-0 flex-1 items-center justify-center gap-2">
        {transcribing ? (
          <>
            <span className="h-3.5 w-3.5 shrink-0 animate-spin rounded-full border-2 border-current border-t-transparent" />
            <span>{transcribingLabel}</span>
          </>
        ) : cancelPending ? (
          <span>{cancelLabel}</span>
        ) : recording ? (
          <>
            <VoiceRecordingIndicator elapsedMs={elapsedMs} analyser={analyser} compact />
            <span className="shrink-0 text-[11px]">{recordingLabel}</span>
          </>
        ) : (
          <>
            <Mic size={17} className="shrink-0" />
            <span>{idleLabel}</span>
          </>
        )}
      </span>
    </motion.button>
  );
}

function formatElapsed(ms: number) {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function VoiceRecordingIndicator({
  elapsedMs,
  analyser,
  frozen = false,
  compact = false,
}: {
  elapsedMs: number;
  analyser: AnalyserNode | null;
  frozen?: boolean;
  compact?: boolean;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const samplesRef = useRef<number[]>([]);
  const frameRef = useRef<number | null>(null);
  const lastPushRef = useRef(0);
  const dataRef = useRef<Uint8Array | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const measure = () => {
      const rect = el.getBoundingClientRect();
      setSize({ w: Math.floor(rect.width), h: Math.floor(rect.height) });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    measure();
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    samplesRef.current = [];
    lastPushRef.current = 0;
    dataRef.current = analyser ? new Uint8Array(new ArrayBuffer(analyser.fftSize)) : null;
  }, [analyser]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || size.w <= 0) return;
    const dpr = window.devicePixelRatio || 1;
    const width = size.w;
    const height = size.h || 20;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);
    const context = canvas.getContext("2d");
    if (!context) return;
    context.scale(dpr, dpr);
    const step = 4;
    const maxBars = Math.ceil(width / step) + 4;

    const draw = (now: number) => {
      if (!frozen && analyser && dataRef.current && now - lastPushRef.current >= 50) {
        analyser.getByteTimeDomainData(dataRef.current as Uint8Array<ArrayBuffer>);
        let sumSq = 0;
        for (const sample of dataRef.current) {
          const value = (sample - 128) / 128;
          sumSq += value * value;
        }
        samplesRef.current.push(Math.min(1, Math.pow(Math.sqrt(sumSq / dataRef.current.length) * 18, 0.5)));
        if (samplesRef.current.length > maxBars) samplesRef.current.splice(0, samplesRef.current.length - maxBars);
        lastPushRef.current = now;
      }
      context.clearRect(0, 0, width, height);
      const centerY = height / 2;
      samplesRef.current.forEach((sample, index, samples) => {
        const age = samples.length - 1 - index;
        const x = width - 2 - age * step;
        if (x < 0) return;
        const barHeight = Math.max(3, sample * (height - 2));
        context.fillStyle = `rgba(244, 252, 248, ${0.95 - (age / Math.max(1, maxBars - 1)) * 0.55})`;
        context.fillRect(x, centerY - barHeight / 2, 2, barHeight);
      });
      frameRef.current = window.requestAnimationFrame(draw);
    };
    frameRef.current = window.requestAnimationFrame(draw);
    return () => {
      if (frameRef.current !== null) window.cancelAnimationFrame(frameRef.current);
    };
  }, [analyser, frozen, size.h, size.w]);

  return (
    <div className={cn("flex min-w-0 items-center gap-2", compact ? "flex-1" : "flex-1 py-2.5")}>
      <div ref={containerRef} className="relative h-5 min-w-0 flex-1 overflow-hidden">
        <div className="pointer-events-none absolute inset-x-0 top-1/2 h-px -translate-y-1/2 bg-fg/15" />
        <canvas ref={canvasRef} className="relative block h-full w-full" />
      </div>
      <span className="shrink-0 text-[11px] tabular-nums text-fg/55">{formatElapsed(elapsedMs)}</span>
    </div>
  );
}
