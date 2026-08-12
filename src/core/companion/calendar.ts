import { invoke } from "@tauri-apps/api/core";

export type CompanionCalendarEventCategory =
  | "traditionalFestival"
  | "solarTerm"
  | "westernHoliday";

export type CompanionCalendarWeekday = 1 | 2 | 3 | 4 | 5 | 6 | 7;

export interface CompanionCalendarEvent {
  id: string;
  name: string;
  date: string;
  weekday: CompanionCalendarWeekday;
  category: CompanionCalendarEventCategory;
  daysUntil: number;
}

export function listUpcomingCompanionCalendarEvents(
  sessionId: string,
  days = 30,
): Promise<CompanionCalendarEvent[]> {
  return invoke<CompanionCalendarEvent[]>("companion_calendar_upcoming", {
    sessionId,
    days,
  });
}
