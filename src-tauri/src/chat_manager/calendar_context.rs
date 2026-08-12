use std::collections::HashSet;

use chrono::{Datelike, Duration, Local, LocalResult, NaiveDate, TimeZone, Weekday};
use serde::Serialize;
use tyme4rs::tyme::solar::SolarDay;

use super::temporal::companion_effective_now;
use super::types::Session;

const DEFAULT_LOOKAHEAD_DAYS: u32 = 30;
const MAX_LOOKAHEAD_DAYS: u32 = 90;

pub const PROMPT_ENTRY_ID: &str = "companion_calendar";
pub const DATE_PLACEHOLDER: &str = "{{calendar_date}}";
pub const LOOKAHEAD_DAYS_PLACEHOLDER: &str = "{{calendar_lookahead_days}}";
pub const EVENTS_PLACEHOLDER: &str = "{{calendar_events}}";
pub const DEFAULT_PROMPT_TEMPLATE: &str = "# 日历上下文\n当前日期：{{calendar_date}}\n未来{{calendar_lookahead_days}}天的节日或节气信息：\n{{calendar_events}}\n\n这些日期用于建立时间与季节背景，不代表用户一定庆祝。仅在日期临近、当天到来或当前话题自然相关时使用，不要主动反复提及。";

const SOLAR_TERMS: [(&str, &str); 24] = [
    ("solarTerm.winterSolstice", "冬至"),
    ("solarTerm.minorCold", "小寒"),
    ("solarTerm.majorCold", "大寒"),
    ("solarTerm.startOfSpring", "立春"),
    ("solarTerm.rainWater", "雨水"),
    ("solarTerm.awakeningOfInsects", "惊蛰"),
    ("solarTerm.springEquinox", "春分"),
    ("solarTerm.clearAndBright", "清明"),
    ("solarTerm.grainRain", "谷雨"),
    ("solarTerm.startOfSummer", "立夏"),
    ("solarTerm.grainBuds", "小满"),
    ("solarTerm.grainInEar", "芒种"),
    ("solarTerm.summerSolstice", "夏至"),
    ("solarTerm.minorHeat", "小暑"),
    ("solarTerm.majorHeat", "大暑"),
    ("solarTerm.startOfAutumn", "立秋"),
    ("solarTerm.endOfHeat", "处暑"),
    ("solarTerm.whiteDew", "白露"),
    ("solarTerm.autumnEquinox", "秋分"),
    ("solarTerm.coldDew", "寒露"),
    ("solarTerm.frostDescent", "霜降"),
    ("solarTerm.startOfWinter", "立冬"),
    ("solarTerm.minorSnow", "小雪"),
    ("solarTerm.majorSnow", "大雪"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CalendarEventCategory {
    TraditionalFestival,
    SolarTerm,
    WesternHoliday,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub name: String,
    pub date: String,
    pub weekday: u32,
    pub category: CalendarEventCategory,
    pub days_until: u32,
    #[serde(skip_serializing)]
    date_value: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarPromptData {
    pub current_date: String,
    pub lookahead_days: u32,
    pub events: String,
}

impl CalendarPromptData {
    pub fn render_default_context(&self) -> String {
        render_prompt_variables(DEFAULT_PROMPT_TEMPLATE, Some(self))
    }
}

impl CalendarEvent {
    fn new(
        id: &str,
        name: &str,
        date: NaiveDate,
        category: CalendarEventCategory,
        days_until: u32,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            date: date.format("%Y-%m-%d").to_string(),
            weekday: date.weekday().number_from_monday(),
            category,
            days_until,
            date_value: date,
        }
    }
}

pub fn calendar_awareness_enabled(session: &Session) -> bool {
    session
        .companion_state
        .as_ref()
        .and_then(|value| value.get("preferences"))
        .and_then(|value| {
            value
                .get("calendarAwarenessEnabled")
                .or_else(|| value.get("calendar_awareness_enabled"))
        })
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

fn disabled_event_ids(session: &Session) -> HashSet<String> {
    session
        .companion_state
        .as_ref()
        .and_then(|value| value.get("preferences"))
        .and_then(|value| {
            value
                .get("calendarDisabledEventIds")
                .or_else(|| value.get("calendar_disabled_event_ids"))
        })
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn reference_date(session: &Session) -> NaiveDate {
    match Local.timestamp_millis_opt(companion_effective_now(session) as i64) {
        LocalResult::Single(value) => value.date_naive(),
        LocalResult::Ambiguous(value, _) => value.date_naive(),
        LocalResult::None => Local::now().date_naive(),
    }
}

pub fn upcoming_events_for_session(session: &Session, days: u32) -> Vec<CalendarEvent> {
    upcoming_events(reference_date(session), days.clamp(1, MAX_LOOKAHEAD_DAYS))
}

pub fn calendar_lookahead_days(session: &Session) -> u32 {
    session
        .companion_state
        .as_ref()
        .and_then(|value| value.get("preferences"))
        .and_then(|value| {
            value
                .get("calendarLookaheadDays")
                .or_else(|| value.get("calendar_lookahead_days"))
        })
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(DEFAULT_LOOKAHEAD_DAYS)
        .clamp(1, MAX_LOOKAHEAD_DAYS)
}

pub fn prompt_data(session: &Session) -> Option<CalendarPromptData> {
    if !calendar_awareness_enabled(session) {
        return None;
    }

    let start = reference_date(session);
    let lookahead_days = calendar_lookahead_days(session);
    let disabled_ids = disabled_event_ids(session);
    let events = upcoming_events(start, lookahead_days)
        .into_iter()
        .filter(|event| !disabled_ids.contains(&event.id))
        .collect::<Vec<_>>();
    if events.is_empty() {
        return None;
    }

    let mut event_lines = Vec::with_capacity(events.len());
    for event in events {
        let timing = if event.days_until == 0 {
            "今天".to_string()
        } else {
            format!("{}天后", event.days_until)
        };
        event_lines.push(format!(
            "- {}：{}（{}，{}）",
            format_chinese_date(event.date_value),
            event.name,
            category_name(event.category),
            timing
        ));
    }

    Some(CalendarPromptData {
        current_date: format_chinese_date(start),
        lookahead_days,
        events: event_lines.join("\n"),
    })
}

pub fn contains_prompt_placeholder(content: &str) -> bool {
    [
        DATE_PLACEHOLDER,
        LOOKAHEAD_DAYS_PLACEHOLDER,
        EVENTS_PLACEHOLDER,
    ]
    .iter()
    .any(|placeholder| content.contains(placeholder))
}

pub fn render_prompt_variables(template: &str, data: Option<&CalendarPromptData>) -> String {
    let date = data.map(|value| value.current_date.as_str()).unwrap_or("");
    let lookahead_days = data
        .map(|value| value.lookahead_days.to_string())
        .unwrap_or_default();
    let events = data.map(|value| value.events.as_str()).unwrap_or("");

    template
        .replace(DATE_PLACEHOLDER, date)
        .replace(LOOKAHEAD_DAYS_PLACEHOLDER, &lookahead_days)
        .replace(EVENTS_PLACEHOLDER, events)
}

fn format_chinese_date(date: NaiveDate) -> String {
    format!(
        "{}年{}月{}日 {}",
        date.year(),
        date.month(),
        date.day(),
        chinese_weekday_name(date.weekday())
    )
}

fn chinese_weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "星期一",
        Weekday::Tue => "星期二",
        Weekday::Wed => "星期三",
        Weekday::Thu => "星期四",
        Weekday::Fri => "星期五",
        Weekday::Sat => "星期六",
        Weekday::Sun => "星期日",
    }
}

fn category_name(category: CalendarEventCategory) -> &'static str {
    match category {
        CalendarEventCategory::TraditionalFestival => "中国传统节日",
        CalendarEventCategory::SolarTerm => "节气",
        CalendarEventCategory::WesternHoliday => "西方节日",
    }
}

fn upcoming_events(start: NaiveDate, days: u32) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    for offset in 0..days {
        let Some(date) = start.checked_add_signed(Duration::days(offset as i64)) else {
            break;
        };
        append_traditional_festivals(&mut events, date, offset);
        append_solar_term(&mut events, date, offset);
        append_western_holidays(&mut events, date, offset);
    }
    events
}

fn append_traditional_festivals(events: &mut Vec<CalendarEvent>, date: NaiveDate, days_until: u32) {
    let solar = SolarDay::from_ymd(
        date.year() as isize,
        date.month() as usize,
        date.day() as usize,
    );
    let lunar = solar.get_lunar_day();
    let lunar_month = lunar.get_month();
    let lunar_day = lunar.get_day();

    if lunar_month > 0 {
        let festival = match (lunar_month, lunar_day) {
            (1, 1) => Some(("traditional.springFestival", "春节")),
            (1, 15) => Some(("traditional.lanternFestival", "元宵节")),
            (5, 5) => Some(("traditional.dragonBoatFestival", "端午节")),
            (7, 7) => Some(("traditional.qixiFestival", "七夕")),
            (8, 15) => Some(("traditional.midAutumnFestival", "中秋节")),
            (9, 9) => Some(("traditional.doubleNinthFestival", "重阳节")),
            (12, 8) => Some(("traditional.labaFestival", "腊八节")),
            _ => None,
        };
        if let Some((id, name)) = festival {
            events.push(CalendarEvent::new(
                id,
                name,
                date,
                CalendarEventCategory::TraditionalFestival,
                days_until,
            ));
        }
    }

    let Some(next_date) = date.succ_opt() else {
        return;
    };
    let next_lunar = SolarDay::from_ymd(
        next_date.year() as isize,
        next_date.month() as usize,
        next_date.day() as usize,
    )
    .get_lunar_day();
    if next_lunar.get_month() == 1 && next_lunar.get_day() == 1 {
        events.push(CalendarEvent::new(
            "traditional.newYearsEve",
            "除夕",
            date,
            CalendarEventCategory::TraditionalFestival,
            days_until,
        ));
    }
}

fn append_solar_term(events: &mut Vec<CalendarEvent>, date: NaiveDate, days_until: u32) {
    let term_day = SolarDay::from_ymd(
        date.year() as isize,
        date.month() as usize,
        date.day() as usize,
    )
    .get_term_day();
    if term_day.get_day_index() != 0 {
        return;
    }

    let index = term_day.get_solar_term().get_index();
    if let Some((id, name)) = SOLAR_TERMS.get(index) {
        events.push(CalendarEvent::new(
            id,
            name,
            date,
            CalendarEventCategory::SolarTerm,
            days_until,
        ));
    }
}

fn append_western_holidays(events: &mut Vec<CalendarEvent>, date: NaiveDate, days_until: u32) {
    let fixed = match (date.month(), date.day()) {
        (2, 14) => Some(("western.valentinesDay", "情人节")),
        (10, 31) => Some(("western.halloween", "万圣夜")),
        (12, 24) => Some(("western.christmasEve", "平安夜")),
        (12, 25) => Some(("western.christmas", "圣诞节")),
        _ => None,
    };
    if let Some((id, name)) = fixed {
        events.push(CalendarEvent::new(
            id,
            name,
            date,
            CalendarEventCategory::WesternHoliday,
            days_until,
        ));
    }

    if easter_date(date.year()) == Some(date) {
        events.push(CalendarEvent::new(
            "western.easter",
            "复活节",
            date,
            CalendarEventCategory::WesternHoliday,
            days_until,
        ));
    }
}

// Meeus/Jones/Butcher Gregorian computus.
fn easter_date(year: i32) -> Option<NaiveDate> {
    let a = year.rem_euclid(19);
    let b = year.div_euclid(100);
    let c = year.rem_euclid(100);
    let d = b.div_euclid(4);
    let e = b.rem_euclid(4);
    let f = (b + 8).div_euclid(25);
    let g = (b - f + 1).div_euclid(3);
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let i = c.div_euclid(4);
    let k = c.rem_euclid(4);
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let m = (a + 11 * h + 22 * l).div_euclid(451);
    let month = (h + l - 7 * m + 114).div_euclid(31);
    let day = (h + l - 7 * m + 114).rem_euclid(31) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_event(events: &[CalendarEvent], id: &str, date: &str) -> bool {
        events
            .iter()
            .any(|event| event.id == id && event.date == date)
    }

    #[test]
    fn includes_lunar_festivals_with_gregorian_dates() {
        let events = upcoming_events(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(), 5);
        assert!(has_event(&events, "traditional.newYearsEve", "2026-02-16"));
        assert!(has_event(
            &events,
            "traditional.springFestival",
            "2026-02-17"
        ));
        let spring_festival = events
            .iter()
            .find(|event| event.id == "traditional.springFestival")
            .unwrap();
        assert_eq!(spring_festival.weekday, 2);
        assert_eq!(
            format_chinese_date(spring_festival.date_value),
            "2026年2月17日 星期二"
        );
    }

    #[test]
    fn includes_exact_solar_terms_in_window() {
        let events = upcoming_events(NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(), 30);
        assert!(has_event(&events, "solarTerm.endOfHeat", "2026-08-23"));
        assert!(has_event(&events, "solarTerm.whiteDew", "2026-09-07"));
    }

    #[test]
    fn calculates_gregorian_easter() {
        assert_eq!(easter_date(2026), NaiveDate::from_ymd_opt(2026, 4, 5));
        assert_eq!(easter_date(2025), NaiveDate::from_ymd_opt(2025, 4, 20));
    }

    #[test]
    fn excludes_the_day_after_the_requested_window() {
        let events = upcoming_events(NaiveDate::from_ymd_opt(2026, 8, 24).unwrap(), 14);
        assert!(!has_event(&events, "solarTerm.whiteDew", "2026-09-07"));
    }

    #[test]
    fn renders_editable_calendar_prompt_variables() {
        let data = CalendarPromptData {
            current_date: "2026年8月12日 星期三".to_string(),
            lookahead_days: 30,
            events: "- 2026年8月23日 星期日：处暑（节气，11天后）".to_string(),
        };
        let template =
            "日期：{{calendar_date}}\n范围：{{calendar_lookahead_days}}天\n{{calendar_events}}";

        assert_eq!(
            render_prompt_variables(template, Some(&data)),
            "日期：2026年8月12日 星期三\n范围：30天\n- 2026年8月23日 星期日：处暑（节气，11天后）"
        );
        assert_eq!(
            data.render_default_context(),
            "# 日历上下文\n当前日期：2026年8月12日 星期三\n未来30天的节日或节气信息：\n- 2026年8月23日 星期日：处暑（节气，11天后）\n\n这些日期用于建立时间与季节背景，不代表用户一定庆祝。仅在日期临近、当天到来或当前话题自然相关时使用，不要主动反复提及。"
        );
    }
}
