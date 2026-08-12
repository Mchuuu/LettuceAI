import { useCallback, useEffect, useMemo, useState } from "react";
import { CalendarDays, Check, Loader2 } from "lucide-react";

import {
  listUpcomingCompanionCalendarEvents,
  type CompanionCalendarEvent,
  type CompanionCalendarWeekday,
} from "../../../../core/companion/calendar";
import { useI18n } from "../../../../core/i18n/context";
import { saveSession } from "../../../../core/storage/repo";
import {
  CompanionSessionStateSchema,
  type CompanionSessionState,
  type Session,
} from "../../../../core/storage/schemas";
import { Switch } from "../../../components/Switch";
import { cn, radius } from "../../../design-tokens";

const LOOKAHEAD_PRESETS = [7, 14, 30] as const;
const MIN_LOOKAHEAD_DAYS = 1;
const MAX_LOOKAHEAD_DAYS = 90;
const WEEKDAY_LABEL_KEYS = {
  1: "chats.settings.calendarWeekdayMonday",
  2: "chats.settings.calendarWeekdayTuesday",
  3: "chats.settings.calendarWeekdayWednesday",
  4: "chats.settings.calendarWeekdayThursday",
  5: "chats.settings.calendarWeekdayFriday",
  6: "chats.settings.calendarWeekdaySaturday",
  7: "chats.settings.calendarWeekdaySunday",
} as const satisfies Record<CompanionCalendarWeekday, string>;

function isLookaheadPreset(days: number): boolean {
  return LOOKAHEAD_PRESETS.some((preset) => preset === days);
}

function lookaheadDraftValue(days: number): string {
  return isLookaheadPreset(days) ? "" : String(days);
}

interface CompanionCalendarSettingsProps {
  session: Session | null;
  onSessionChange: (session: Session) => void;
}

export function CompanionCalendarSettings({
  session,
  onSessionChange,
}: CompanionCalendarSettingsProps) {
  const { t } = useI18n();
  const [events, setEvents] = useState<CompanionCalendarEvent[]>([]);
  const [eventsLoading, setEventsLoading] = useState(false);
  const [eventsFailed, setEventsFailed] = useState(false);
  const [savingPreference, setSavingPreference] = useState(false);

  const preferences = session?.companionState?.preferences;
  const enabled = preferences?.calendarAwarenessEnabled ?? true;
  const lookaheadDays = preferences?.calendarLookaheadDays ?? 30;
  const timeOverride = preferences?.timeOverride;
  const disabledEventIds = useMemo(
    () => new Set(preferences?.calendarDisabledEventIds ?? []),
    [preferences?.calendarDisabledEventIds],
  );
  const [lookaheadDraft, setLookaheadDraft] = useState(() => lookaheadDraftValue(lookaheadDays));

  useEffect(() => {
    setLookaheadDraft(lookaheadDraftValue(lookaheadDays));
  }, [lookaheadDays]);

  useEffect(() => {
    const sessionId = session?.id;
    if (!sessionId) {
      setEvents([]);
      setEventsLoading(false);
      setEventsFailed(false);
      return;
    }

    let active = true;
    setEventsLoading(true);
    setEventsFailed(false);
    void listUpcomingCompanionCalendarEvents(sessionId, lookaheadDays)
      .then((nextEvents) => {
        if (active) {
          setEvents(nextEvents);
        }
      })
      .catch((error) => {
        console.error("Failed to load companion calendar events:", error);
        if (active) {
          setEvents([]);
          setEventsFailed(true);
        }
      })
      .finally(() => {
        if (active) {
          setEventsLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [
    lookaheadDays,
    session?.id,
    timeOverride?.anchorMs,
    timeOverride?.mode,
    timeOverride?.setAtMs,
  ]);

  const updatePreferences = useCallback(
    async (patch: Partial<CompanionSessionState["preferences"]>) => {
      if (!session || savingPreference) {
        return false;
      }

      const nextCompanionState = CompanionSessionStateSchema.parse({
        ...(session.companionState ?? {}),
        preferences: {
          ...(session.companionState?.preferences ?? {}),
          ...patch,
        },
        updatedAt: Date.now(),
      });
      const updatedSession: Session = {
        ...session,
        companionState: nextCompanionState,
        updatedAt: Date.now(),
      };

      setSavingPreference(true);
      try {
        await saveSession(updatedSession);
        onSessionChange(updatedSession);
        return true;
      } catch (error) {
        console.error("Failed to update companion calendar preferences:", error);
        return false;
      } finally {
        setSavingPreference(false);
      }
    },
    [onSessionChange, savingPreference, session],
  );

  const toggleEvent = useCallback(
    (eventId: string) => {
      const nextDisabledIds = new Set(disabledEventIds);
      if (nextDisabledIds.has(eventId)) {
        nextDisabledIds.delete(eventId);
      } else {
        nextDisabledIds.add(eventId);
      }
      void updatePreferences({
        calendarDisabledEventIds: Array.from(nextDisabledIds).sort(),
      });
    },
    [disabledEventIds, updatePreferences],
  );

  const setLookahead = useCallback(
    (days: number) => {
      const nextDays = Math.min(
        MAX_LOOKAHEAD_DAYS,
        Math.max(MIN_LOOKAHEAD_DAYS, Math.round(days)),
      );
      setLookaheadDraft(lookaheadDraftValue(nextDays));
      if (nextDays !== lookaheadDays) {
        setEventsFailed(false);
        setEventsLoading(true);
        void updatePreferences({ calendarLookaheadDays: nextDays }).then((saved) => {
          if (!saved) {
            setEventsLoading(false);
          }
        });
      }
    },
    [lookaheadDays, updatePreferences],
  );

  const commitLookaheadDraft = useCallback(() => {
    const parsed = Number.parseInt(lookaheadDraft, 10);
    if (!Number.isFinite(parsed)) {
      setLookaheadDraft(lookaheadDraftValue(lookaheadDays));
      return;
    }
    setLookahead(parsed);
  }, [lookaheadDraft, lookaheadDays, setLookahead]);

  const controlsDisabled = !session || savingPreference;

  return (
    <>
      <div
        className={cn(
          "flex items-start justify-between gap-3 rounded-xl border px-4 py-3",
          !session
            ? "border-white/5 bg-[#0c0d13]/50 opacity-50 cursor-not-allowed"
            : "border-white/10 bg-[#0c0d13]/85",
        )}
      >
        <div className="flex min-w-0 items-start gap-3">
          <div
            className={cn(
              "mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center border border-fg/15 bg-fg/10 text-fg/75",
              radius.full,
            )}
          >
            <CalendarDays className="h-4 w-4" />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold text-white">
              {t("chats.settings.calendarAwareness")}
            </p>
            <p className="mt-1 text-xs text-white/50">
              {session
                ? t("chats.settings.calendarAwarenessDesc")
                : t("chats.settings.openChatSessionFirst")}
            </p>
          </div>
        </div>
        <Switch
          id="companion-calendar-awareness"
          checked={enabled}
          onChange={() => void updatePreferences({ calendarAwarenessEnabled: !enabled })}
          disabled={controlsDisabled}
        />
      </div>

      <div className="rounded-xl border border-white/10 bg-[#0c0d13]/85 px-4 py-3">
        <div className="mb-3">
          <p className="text-sm font-semibold text-white">
            {t("chats.settings.calendarLookahead")}
          </p>
          <p className="mt-1 text-xs text-white/50">
            {t("chats.settings.calendarLookaheadDesc")}
          </p>
        </div>
        <div className="grid grid-cols-3 gap-2">
          {LOOKAHEAD_PRESETS.map((days) => (
            <button
              key={days}
              type="button"
              disabled={controlsDisabled}
              onClick={() => setLookahead(days)}
              className={cn(
                "h-9 rounded-lg border text-xs font-medium transition-colors",
                lookaheadDays === days
                  ? "border-white/25 bg-white/15 text-white"
                  : "border-white/10 bg-white/5 text-white/55 hover:bg-white/10 hover:text-white/80",
                controlsDisabled && "cursor-not-allowed opacity-50",
              )}
            >
              {days} {t("chats.settings.calendarDaysUnit")}
            </button>
          ))}
        </div>
        <div className="mt-2 flex items-center gap-2">
          <span className="shrink-0 text-xs text-white/45">
            {t("chats.settings.calendarCustomDays")}
          </span>
          <label
            className={cn(
              "flex h-9 min-w-0 flex-1 items-center rounded-lg border px-3 transition-colors",
              !isLookaheadPreset(lookaheadDays)
                ? "border-white/25 bg-white/15"
                : "border-white/10 bg-white/5",
              controlsDisabled && "opacity-50",
            )}
          >
            <input
              type="number"
              inputMode="numeric"
              min={MIN_LOOKAHEAD_DAYS}
              max={MAX_LOOKAHEAD_DAYS}
              value={lookaheadDraft}
              disabled={controlsDisabled}
              aria-label={t("chats.settings.calendarCustomDays")}
              placeholder={`${MIN_LOOKAHEAD_DAYS}-${MAX_LOOKAHEAD_DAYS}`}
              onChange={(event) =>
                setLookaheadDraft(event.target.value.replace(/\D/g, "").slice(0, 2))
              }
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  commitLookaheadDraft();
                }
              }}
              className="min-w-0 flex-1 bg-transparent text-center text-xs font-medium text-white outline-none placeholder:text-white/25"
            />
            <span className="shrink-0 text-[10px] text-white/45">
              {t("chats.settings.calendarDaysUnit")}
            </span>
          </label>
          <button
            type="button"
            disabled={controlsDisabled || lookaheadDraft.length === 0}
            onClick={commitLookaheadDraft}
            aria-label={t("chats.settings.calendarApplyCustomDays")}
            title={t("chats.settings.calendarApplyCustomDays")}
            className={cn(
              "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border transition-colors",
              "border-white/10 bg-white/5 text-white/60 hover:bg-white/10 hover:text-white",
              (controlsDisabled || lookaheadDraft.length === 0) &&
                "cursor-not-allowed opacity-40",
            )}
          >
            <Check className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="overflow-hidden rounded-xl border border-white/10 bg-[#0c0d13]/85">
        <div className="border-b border-white/8 px-4 py-3">
          <p className="text-sm font-semibold text-white">
            {t("chats.settings.upcomingCalendarEvents", { count: lookaheadDays })}
          </p>
          <p className="mt-1 text-xs text-white/50">
            {t("chats.settings.upcomingCalendarEventsDesc")}
          </p>
        </div>
        <div className="relative">
          {!session ? (
            <p className="px-4 py-5 text-center text-xs text-white/45">
              {t("chats.settings.openChatSessionFirst")}
            </p>
          ) : eventsFailed ? (
            <p className="px-4 py-5 text-center text-xs text-red-300/80">
              {t("chats.settings.calendarEventsLoadFailed")}
            </p>
          ) : events.length === 0 ? (
            eventsLoading ? (
              <div className="flex min-h-20 items-center justify-center gap-2 px-4 py-5 text-xs text-white/50">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>{t("chats.settings.calendarEventsLoading")}</span>
              </div>
            ) : (
              <p className="px-4 py-5 text-center text-xs text-white/45">
                {t("chats.settings.noUpcomingCalendarEvents")}
              </p>
            )
          ) : (
            <div className={cn(!enabled && "opacity-70")}>
              {events.map((event, index) => {
                const categoryLabel =
                  event.category === "traditionalFestival"
                    ? t("chats.settings.calendarCategoryTraditional")
                    : event.category === "solarTerm"
                      ? t("chats.settings.calendarCategorySolarTerm")
                      : t("chats.settings.calendarCategoryWestern");
                const relativeDate =
                  event.daysUntil === 0
                    ? t("chats.settings.calendarToday")
                    : t("chats.settings.calendarDaysUntil", { count: event.daysUntil });
                const [year, month, day] = event.date.split("-").map(Number);
                const displayDate = t("chats.settings.calendarEventDate", {
                  date: event.date,
                  year,
                  month,
                  day,
                  weekday: t(WEEKDAY_LABEL_KEYS[event.weekday]),
                });
                return (
                  <div
                    key={`${event.id}-${event.date}`}
                    className={cn(
                      "flex min-h-16 items-center justify-between gap-3 px-4 py-3",
                      index > 0 && "border-t border-white/8",
                    )}
                  >
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="text-sm font-medium text-white">{event.name}</p>
                        <span className="rounded border border-white/10 bg-white/5 px-1.5 py-0.5 text-[10px] text-white/50">
                          {categoryLabel}
                        </span>
                      </div>
                      <p className="mt-1 text-xs text-white/45">
                        {displayDate} · {relativeDate}
                      </p>
                    </div>
                    <Switch
                      id={`companion-calendar-event-${event.id}`}
                      checked={!disabledEventIds.has(event.id)}
                      onChange={() => toggleEvent(event.id)}
                      disabled={controlsDisabled}
                    />
                  </div>
                );
              })}
            </div>
          )}
          {session && eventsLoading && events.length > 0 ? (
            <div
              className="absolute inset-0 z-10 flex items-center justify-center bg-[#0c0d13]/70"
              role="status"
              aria-label={t("chats.settings.calendarEventsLoading")}
            >
              <Loader2 className="h-5 w-5 animate-spin text-white/75" />
              <span className="sr-only">{t("chats.settings.calendarEventsLoading")}</span>
            </div>
          ) : null}
        </div>
      </div>
    </>
  );
}
