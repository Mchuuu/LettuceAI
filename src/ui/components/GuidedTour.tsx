import {
  type ReactNode,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { AnimatePresence, motion } from "framer-motion";
import { ChevronsRight, SendHorizonal, Square } from "lucide-react";

import { hasSeenTooltip, setTooltipSeen } from "../../core/storage/appState";
import { useI18n, type TranslationKey } from "../../core/i18n/context";

type TourStep = {
  id: string;
  targetAttr: string;
  titleKey?: TranslationKey;
  bodyKey?: TranslationKey;
  title?: string;
  body?: string;
  extra?: ReactNode;
};

type TourConfig = {
  storageKey: string;
  steps: TourStep[];
};

export type TourId =
  | "appShell"
  | "chatDetail"
  | "postFirstMessage"
  | "speechRecognition";

const TOURS: Record<TourId, TourConfig> = {
  appShell: {
    storageKey: "app_tour_v1",
    steps: [
      {
        id: "chats",
        targetAttr: "nav-chats",
        titleKey: "tour.appShell.chats.title",
        bodyKey: "tour.appShell.chats.body",
      },
      {
        id: "groups",
        targetAttr: "nav-groups",
        titleKey: "tour.appShell.groups.title",
        bodyKey: "tour.appShell.groups.body",
      },
      {
        id: "discover",
        targetAttr: "nav-discover",
        titleKey: "tour.appShell.discover.title",
        bodyKey: "tour.appShell.discover.body",
      },
      {
        id: "library",
        targetAttr: "nav-library",
        titleKey: "tour.appShell.library.title",
        bodyKey: "tour.appShell.library.body",
      },
      {
        id: "settings",
        targetAttr: "top-settings",
        titleKey: "tour.appShell.settings.title",
        bodyKey: "tour.appShell.settings.body",
      },
      {
        id: "search",
        targetAttr: "top-search",
        titleKey: "tour.appShell.search.title",
        bodyKey: "tour.appShell.search.body",
      },
      {
        id: "create",
        targetAttr: "nav-create",
        titleKey: "tour.appShell.create.title",
        bodyKey: "tour.appShell.create.body",
      },
    ],
  },

  chatDetail: {
    storageKey: "chat_detail_tour_v1",
    steps: [
      {
        id: "chat-title",
        targetAttr: "chat-title",
        titleKey: "tour.chatDetail.chatTitle.title",
        bodyKey: "tour.chatDetail.chatTitle.body",
      },
      {
        id: "chat-memory",
        targetAttr: "chat-memory",
        titleKey: "tour.chatDetail.chatMemory.title",
        bodyKey: "tour.chatDetail.chatMemory.body",
      },
      {
        id: "chat-search",
        targetAttr: "chat-search",
        titleKey: "tour.chatDetail.chatSearch.title",
        bodyKey: "tour.chatDetail.chatSearch.body",
      },
      {
        id: "chat-lorebook",
        targetAttr: "chat-lorebook",
        titleKey: "tour.chatDetail.chatLorebook.title",
        bodyKey: "tour.chatDetail.chatLorebook.body",
      },
      {
        id: "chat-plus",
        targetAttr: "chat-plus",
        titleKey: "tour.chatDetail.chatPlus.title",
        bodyKey: "tour.chatDetail.chatPlus.body",
      },
      {
        id: "chat-composer",
        targetAttr: "chat-composer",
        titleKey: "tour.chatDetail.chatComposer.title",
        bodyKey: "tour.chatDetail.chatComposer.body",
      },
      {
        id: "chat-send",
        targetAttr: "chat-send",
        titleKey: "tour.chatDetail.chatSend.title",
        bodyKey: "tour.chatDetail.chatSend.body",
        extra: <SendButtonStates />,
      },
    ],
  },

  speechRecognition: {
    storageKey: "asr_tour_v1",
    steps: [
      {
        id: "asr-active-model",
        targetAttr: "asr-active-model",
        titleKey: "tour.speechRecognition.activeModel.title",
        bodyKey: "tour.speechRecognition.activeModel.body",
      },
      {
        id: "asr-library",
        targetAttr: "asr-library",
        titleKey: "tour.speechRecognition.library.title",
        bodyKey: "tour.speechRecognition.library.body",
      },
      {
        id: "asr-mic-test",
        targetAttr: "asr-mic-test",
        titleKey: "tour.speechRecognition.micTest.title",
        bodyKey: "tour.speechRecognition.micTest.body",
      },
      {
        id: "asr-runtime",
        targetAttr: "asr-runtime",
        titleKey: "tour.speechRecognition.runtime.title",
        bodyKey: "tour.speechRecognition.runtime.body",
      },
    ],
  },

  postFirstMessage: {
    storageKey: "post_first_message_tour_v1",
    steps: [
      {
        id: "chat-regenerate",
        targetAttr: "chat-regenerate",
        titleKey: "tour.postFirstMessage.chatRegenerate.title",
        bodyKey: "tour.postFirstMessage.chatRegenerate.body",
      },
      {
        id: "chat-variants",
        targetAttr: "chat-variants",
        titleKey: "tour.postFirstMessage.chatVariants.title",
        bodyKey: "tour.postFirstMessage.chatVariants.body",
      },
      {
        id: "chat-long-press",
        targetAttr: "chat-message-bubble",
        titleKey: "tour.postFirstMessage.chatLongPress.title",
        bodyKey: "tour.postFirstMessage.chatLongPress.body",
      },
    ],
  },
};

const LEGACY_TOOLTIP_KEY = "create_button";

const SPOTLIGHT_PAD = 8;
const CARD_GAP = 16;
const EDGE_PAD = 16;

type Rect = { left: number; top: number; width: number; height: number };

function getViewport() {
  if (typeof window === "undefined") return { w: 0, h: 0 };
  return { w: window.innerWidth, h: window.innerHeight };
}

export function GuidedTour({
  tour,
  onDismiss,
}: {
  tour: TourId;
  onDismiss: () => void;
}) {
  const { t } = useI18n();
  const config = TOURS[tour];
  const steps = config.steps;
  const totalSteps = steps.length;

  const [stepIdx, setStepIdx] = useState(0);
  const [rect, setRect] = useState<Rect | null>(null);
  const [viewport, setViewport] = useState(getViewport);
  const [cardSize, setCardSize] = useState({ width: 320, height: 0 });
  const [cardMeasured, setCardMeasured] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  const step = steps[stepIdx];
  const isLastStep = stepIdx === totalSteps - 1;

  useEffect(() => {
    window.dispatchEvent(
      new CustomEvent("tour:step", { detail: { tour, stepId: step?.id ?? null } }),
    );
    return () => {
      window.dispatchEvent(
        new CustomEvent("tour:step", { detail: { tour, stepId: null } }),
      );
    };
  }, [tour, step?.id]);

  const finish = useCallback(() => {
    void setTooltipSeen(config.storageKey, true);
    onDismiss();
  }, [config.storageKey, onDismiss]);

  const next = useCallback(() => {
    if (isLastStep) {
      finish();
    } else {
      setStepIdx((i) => i + 1);
    }
  }, [isLastStep, finish]);

  useLayoutEffect(() => {
    if (!step) return;

    let raf = 0;
    let retryTimer = 0;
    let retryCount = 0;
    const MAX_RETRIES = 20;

    const measure = () => {
      const el = document.querySelector<HTMLElement>(
        `[data-tour-id="${step.targetAttr}"]`,
      );
      if (!el) {
        if (retryCount < MAX_RETRIES) {
          retryCount++;
          retryTimer = window.setTimeout(measure, 200);
          return;
        }
        if (stepIdx >= totalSteps - 1) {
          finish();
        } else {
          setStepIdx((i) => i + 1);
        }
        return;
      }
      const r = el.getBoundingClientRect();
      setRect({ left: r.left, top: r.top, width: r.width, height: r.height });
      setViewport(getViewport());
    };

    const scrollIntoViewIfNeeded = () => {
      const el = document.querySelector<HTMLElement>(
        `[data-tour-id="${step.targetAttr}"]`,
      );
      if (!el) return;
      const r = el.getBoundingClientRect();
      const vh = window.innerHeight;
      const margin = 80;
      const offscreen = r.bottom < margin || r.top > vh - margin;
      if (offscreen) {
        el.scrollIntoView({ behavior: "smooth", block: "center" });
      }
    };

    raf = requestAnimationFrame(() => {
      scrollIntoViewIfNeeded();
      measure();
    });

    const onResize = () => measure();
    const onScroll = () => measure();
    window.addEventListener("resize", onResize);
    document.addEventListener("scroll", onScroll, true);
    const ro = new ResizeObserver(() => measure());
    ro.observe(document.documentElement);

    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(retryTimer);
      window.removeEventListener("resize", onResize);
      document.removeEventListener("scroll", onScroll, true);
      ro.disconnect();
    };
  }, [stepIdx, step, finish, totalSteps]);

  const hasRect = rect != null;
  useLayoutEffect(() => {
    if (!cardRef.current) return;
    const el = cardRef.current;
    const update = () => {
      const w = el.offsetWidth;
      const h = el.offsetHeight;
      if (w > 0 && h > 0) {
        setCardSize({ width: w, height: h });
        setCardMeasured(true);
      }
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, [stepIdx, hasRect]);

  if (!step || !rect) return null;

  const hole = {
    x: Math.max(0, rect.left - SPOTLIGHT_PAD),
    y: Math.max(0, rect.top - SPOTLIGHT_PAD),
    w: rect.width + SPOTLIGHT_PAD * 2,
    h: rect.height + SPOTLIGHT_PAD * 2,
  };

  const targetCenterY = rect.top + rect.height / 2;
  const placeAbove = targetCenterY > viewport.h / 2;

  const desiredCenterX = rect.left + rect.width / 2;
  const halfW = cardSize.width / 2;
  const minCenterX = EDGE_PAD + halfW;
  const maxCenterX = Math.max(minCenterX, viewport.w - EDGE_PAD - halfW);
  const clampedCenterX = Math.min(maxCenterX, Math.max(minCenterX, desiredCenterX));
  const cardLeft = clampedCenterX - halfW;

  const cardTopRaw = placeAbove
    ? hole.y - CARD_GAP - cardSize.height
    : hole.y + hole.h + CARD_GAP;
  const cardTop = Math.max(
    EDGE_PAD,
    Math.min(viewport.h - EDGE_PAD - cardSize.height, cardTopRaw),
  );

  const spring = { type: "spring", damping: 26, stiffness: 220 } as const;

  return (
    <AnimatePresence>
      <motion.div
        key={`guided-tour-${tour}`}
        className="fixed inset-0 z-50"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.25 }}
      >
        <svg
          className="pointer-events-none absolute inset-0 h-full w-full"
          aria-hidden
        >
          <defs>
            <mask id={`guided-tour-mask-${tour}`}>
              <rect width="100%" height="100%" fill="white" />
              <motion.rect
                initial={false}
                animate={{ x: hole.x, y: hole.y, width: hole.w, height: hole.h }}
                transition={spring}
                rx={14}
                ry={14}
                fill="black"
              />
            </mask>
            <filter
              id={`guided-tour-glow-${tour}`}
              x="-50%"
              y="-50%"
              width="200%"
              height="200%"
            >
              <feGaussianBlur stdDeviation="6" />
            </filter>
          </defs>
          <rect
            width="100%"
            height="100%"
            fill="rgba(0,0,0,0.74)"
            mask={`url(#guided-tour-mask-${tour})`}
          />
          <motion.rect
            initial={false}
            animate={{ x: hole.x, y: hole.y, width: hole.w, height: hole.h }}
            transition={spring}
            rx={14}
            ry={14}
            fill="none"
            stroke="#34d399"
            strokeOpacity={0.55}
            strokeWidth={4}
            filter={`url(#guided-tour-glow-${tour})`}
          />
          <motion.rect
            initial={false}
            animate={{ x: hole.x, y: hole.y, width: hole.w, height: hole.h }}
            transition={spring}
            rx={14}
            ry={14}
            fill="none"
            stroke="#34d399"
            strokeOpacity={0.9}
            strokeWidth={1.5}
          />
        </svg>

        <div className="absolute inset-0" />

        <motion.div
          className="absolute"
          style={{ top: 0, left: 0 }}
          initial={false}
          animate={{
            x: cardLeft,
            y: cardTop,
            opacity: cardMeasured ? 1 : 0,
          }}
          transition={{
            x: spring,
            y: spring,
            opacity: { duration: 0.2 },
          }}
        >
          <div
            ref={cardRef}
            className="w-[calc(100vw-32px)] max-w-xs overflow-hidden rounded-2xl border border-fg/12 bg-nav/95 backdrop-blur-xl shadow-[0_24px_56px_rgba(0,0,0,0.65),0_0_0_1px_rgba(255,255,255,0.04)_inset]"
          >
            <div className="px-5 pt-4 pb-5">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-fg/40">
                  {t("tour.stepCounter", { current: stepIdx + 1, total: totalSteps })}
                </span>
                <StepDots total={totalSteps} current={stepIdx} />
              </div>

              <motion.div
                key={step.id}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.22, delay: 0.04 }}
                className="mt-3"
              >
                <h3 className="text-[15px] font-semibold leading-snug tracking-tight text-fg">
                  {step.titleKey ? t(step.titleKey) : step.title}
                </h3>
                <p className="mt-1.5 text-[13px] leading-relaxed text-fg/60">
                  {step.bodyKey ? t(step.bodyKey) : step.body}
                </p>
                {step.extra}
              </motion.div>

              <div className="mt-5 flex items-center justify-between">
                <button
                  type="button"
                  onClick={finish}
                  className="text-[11px] font-medium text-fg/40 transition-all duration-150 hover:text-fg/75"
                >
                  {t("tour.skipTour")}
                </button>
                <button
                  type="button"
                  onClick={next}
                  className="rounded-full border border-accent/40 bg-accent/15 px-4 py-1.5 text-[12px] font-semibold text-accent transition-all duration-150 hover:bg-accent/25 active:scale-[0.98]"
                >
                  {isLastStep ? t("tour.gotIt") : t("tour.next")}
                </button>
              </div>
            </div>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}

function StepDots({ total, current }: { total: number; current: number }) {
  return (
    <span className="flex items-center gap-1">
      {Array.from({ length: total }).map((_, i) => (
        <span
          key={i}
          className={`h-1 rounded-full transition-all duration-200 ${
            i === current ? "w-4 bg-accent" : "w-1 bg-fg/20"
          }`}
        />
      ))}
    </span>
  );
}

function SendButtonStates() {
  const { t } = useI18n();
  const states: Array<{
    label: string;
    desc: string;
    icon: ReactNode;
    swatch: string;
  }> = [
    {
      label: t("tour.sendButton.continue.label"),
      desc: t("tour.sendButton.continue.desc"),
      icon: <ChevronsRight size={15} />,
      swatch: "border-fg/15 bg-fg/10 text-fg/70",
    },
    {
      label: t("tour.sendButton.send.label"),
      desc: t("tour.sendButton.send.desc"),
      icon: <SendHorizonal size={15} />,
      swatch: "border-emerald-400/40 bg-emerald-400/20 text-emerald-100",
    },
    {
      label: t("tour.sendButton.sending.label"),
      desc: t("tour.sendButton.sending.desc"),
      icon: (
        <span className="h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent" />
      ),
      swatch: "border-fg/15 bg-fg/10 text-fg/50",
    },
    {
      label: t("tour.sendButton.stop.label"),
      desc: t("tour.sendButton.stop.desc"),
      icon: <Square size={12} fill="currentColor" />,
      swatch: "border-red-400/40 bg-red-400/20 text-red-100",
    },
  ];

  return (
    <div className="mt-3 space-y-2">
      {states.map((state) => (
        <div key={state.label} className="flex items-start gap-3">
          <div
            className={`flex h-7 w-8 shrink-0 items-center justify-center rounded-full border ${state.swatch}`}
          >
            {state.icon}
          </div>
          <div className="min-w-0 flex-1 pt-0.5">
            <div className="text-[11px] font-semibold text-fg/85">
              {state.label}
            </div>
            <div className="text-[11px] leading-snug text-fg/50">
              {state.desc}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

export function useGuidedTour(tour: TourId) {
  const config = TOURS[tour];
  const storageKey = config.storageKey;
  const [shouldShow, setShouldShow] = useState(false);

  const isEventDriven = tour === "postFirstMessage";

  useEffect(() => {
    if (isEventDriven) return;
    let cancelled = false;

    (async () => {
      if (tour === "appShell") {
        await setTooltipSeen(LEGACY_TOOLTIP_KEY, false);
      }

      const seen = await hasSeenTooltip(storageKey);
      if (!cancelled && !seen) {
        setShouldShow(true);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [tour, storageKey, isEventDriven]);

  const dismiss = useCallback(() => {
    setShouldShow(false);
    void setTooltipSeen(storageKey, true);
  }, [storageKey]);

  const show = useCallback(() => {
    void (async () => {
      const seen = await hasSeenTooltip(storageKey);
      if (!seen) setShouldShow(true);
    })();
  }, [storageKey]);

  useEffect(() => {
    if (typeof window === "undefined") return;

    type DebugShape = {
      tourResets?: Record<string, () => Promise<void>>;
      resetTour?: (id: string) => Promise<void>;
      resetAllTours?: () => Promise<void>;
      [key: string]: unknown;
    };

    const globalWindow = window as unknown as { __debug?: DebugShape };
    const debug = (globalWindow.__debug = globalWindow.__debug ?? {});
    const resets = (debug.tourResets = debug.tourResets ?? {});

    resets[tour] = async () => {
      await setTooltipSeen(storageKey, false);
      setShouldShow(false);
      await Promise.resolve();
      setShouldShow(true);
      // eslint-disable-next-line no-console
      console.info(`[GuidedTour:${tour}] reset.`);
    };

    debug.resetTour = async (id: string) => {
      const fn = debug.tourResets?.[id];
      if (fn) {
        await fn();
      } else {
        // eslint-disable-next-line no-console
        console.warn(
          `[GuidedTour] no tour registered for id: ${id}. Known: ${Object.keys(debug.tourResets ?? {}).join(", ")}`,
        );
      }
    };
    debug.resetAllTours = async () => {
      const fns = Object.values(debug.tourResets ?? {});
      for (const fn of fns) {
        await fn();
      }
    };

    return () => {
      if (debug.tourResets) {
        delete debug.tourResets[tour];
      }
    };
  }, [tour, storageKey]);

  return { shouldShow, dismiss, show };
}
