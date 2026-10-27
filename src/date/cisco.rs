//! Cisco timestamp formats: IOS-XR's year-less per-command banner and the
//! year-bearing timestamp printed by "show clock" on IOS/IOS-XE/NX-OS.

use once_cell::sync::Lazy;
use regex::Regex;

use super::{CaptureDate, ParsedDate};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: &str = "Mon|Tue|Wed|Thu|Fri|Sat|Sun";

/// IOS-XR timestamp line, with no year. The exact shape depends on the
/// device's `service timestamps` configuration, so the weekday and the
/// trailing timezone/millisecond fields are all optional, e.g.:
///   Mon Oct 19 02:45:08.742 UTC   (auto-printed command banner)
///   Oct 27 20:33:15               (service timestamps log datetime)
///   Oct 27 20:33:15.826 UTC       (... + msec + show-timezone)
///
/// Note this does *not* cover `service timestamps log uptime`, which
/// replaces the date with elapsed router uptime (e.g. "00:05:23:") - there
/// is no real calendar date to extract from that format, so such lines are
/// correctly left unmatched and the caller's `fallback_date` is used.
///
/// Not anchored at the start: a queued-command echo can leave stray text
/// stuck to the front of this line (e.g. "showMon Oct 19 ... UTC"), and raw
/// syslog lines are prefixed with a node-id like "RP/0/RSP0/CPU0:" whose
/// path segments vary by chassis/role - both are tolerated as leading
/// garbage the same way. We only require the timestamp to occupy the *end*
/// of the line, so real output lines that merely contain a date-shaped
/// substring followed by more text (e.g. actual log messages) won't match.
static XR_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?:(?:{WEEKDAYS})\s+)?(?P<mon>{months})\s+(?P<day>\d{{1,2}})\s+\d{{2}}:\d{{2}}:\d{{2}}(?:\.\d+)?(?:\s+\S+)?\s*$",
        months = MONTHS.join("|"),
    ))
    .unwrap()
});

/// Typical "show clock" output on IOS/IOS-XE/NX-OS, with a year, e.g.
///   *02:45:08.742 UTC Mon Oct 19 2026
///
/// Also not anchored at the start, for the same reason as `XR_DATE_RE`.
static CLOCK_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"\*?\d{{2}}:\d{{2}}:\d{{2}}(?:\.\d+)?\s+\S+\s+(?:{WEEKDAYS})\s+(?P<mon>{months})\s+(?P<day>\d{{1,2}})\s+(?P<year>\d{{4}})\s*$",
        months = MONTHS.join("|"),
    ))
    .unwrap()
});

fn month_num(name: &str) -> Option<u32> {
    MONTHS.iter().position(|m| *m == name).map(|i| (i + 1) as u32)
}

/// Try to parse a Cisco-style timestamp line.
pub fn try_parse_date(line: &str, year_hint: i32) -> Option<ParsedDate> {
    let stripped = line.trim();
    if let Some(caps) = CLOCK_DATE_RE.captures(stripped) {
        let date = CaptureDate {
            year: caps["year"].parse().ok()?,
            month: month_num(&caps["mon"])?,
            day: caps["day"].parse().ok()?,
        };
        return Some(ParsedDate::Explicit(date, caps[0].to_string()));
    }
    if let Some(caps) = XR_DATE_RE.captures(stripped) {
        let date = CaptureDate {
            year: year_hint,
            month: month_num(&caps["mon"])?,
            day: caps["day"].parse().ok()?,
        };
        return Some(ParsedDate::YearInferred(date, caps[0].to_string()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_clock_banner() {
        let parsed = try_parse_date("*02:45:08.742 UTC Mon Oct 19 2026", 1999).unwrap();
        assert!(parsed.had_explicit_year());
        assert_eq!(
            parsed.date(),
            CaptureDate {
                year: 2026,
                month: 10,
                day: 19
            }
        );
    }

    #[test]
    fn parses_year_inferred_xr_banner() {
        let parsed = try_parse_date("Mon Oct 19 02:45:08.742 UTC", 2026).unwrap();
        assert!(!parsed.had_explicit_year());
        assert_eq!(
            parsed.date(),
            CaptureDate {
                year: 2026,
                month: 10,
                day: 19
            }
        );
    }

    #[test]
    fn ignores_non_date_lines() {
        assert!(try_parse_date("interface GigabitEthernet0/0/0/0", 2026).is_none());
    }

    #[test]
    fn ignores_juniper_timestamp_line() {
        assert!(try_parse_date("Current time: 2026-10-22 14:30:00 UTC", 2026).is_none());
    }

    #[test]
    fn parses_bare_datetime_with_no_weekday_or_timezone() {
        // `service timestamps log datetime` with neither msec nor show-timezone.
        let parsed = try_parse_date("Oct 27 20:33:15", 2026).unwrap();
        assert!(!parsed.had_explicit_year());
        assert_eq!(
            parsed.date(),
            CaptureDate {
                year: 2026,
                month: 10,
                day: 27
            }
        );
    }

    #[test]
    fn parses_datetime_with_msec_and_timezone_but_no_weekday() {
        // `service timestamps log datetime msec show-timezone`.
        let parsed = try_parse_date("Oct 27 20:33:15.826 UTC", 2026).unwrap();
        assert_eq!(
            parsed.date(),
            CaptureDate {
                year: 2026,
                month: 10,
                day: 27
            }
        );
    }

    #[test]
    fn parses_syslog_line_with_node_id_prefix() {
        // Raw IOS-XR syslog lines prepend the active RP's node id; the
        // path segment names vary by chassis/role, and the line ends with
        // a bare colon instead of a timezone when none is configured.
        let parsed = try_parse_date("RP/0/RSP0/CPU0:Oct 27 20:33:15.826 :", 2026).unwrap();
        assert_eq!(
            parsed.date(),
            CaptureDate {
                year: 2026,
                month: 10,
                day: 27
            }
        );
    }

    #[test]
    fn parses_syslog_line_with_differently_shaped_node_id() {
        let parsed = try_parse_date("RP/0/RP0/CPU0:Oct 27 20:33:15 :", 2026).unwrap();
        assert_eq!(
            parsed.date(),
            CaptureDate {
                year: 2026,
                month: 10,
                day: 27
            }
        );
    }

    #[test]
    fn ignores_uptime_format_with_no_real_date() {
        // `service timestamps log uptime` replaces the date entirely with
        // elapsed router uptime - there's no calendar date to recover here,
        // so this must fall back to the caller's fallback_date instead.
        assert!(try_parse_date("00:05:23: %SYS-5-CONFIG_I: Configured from console", 2026).is_none());
    }

    #[test]
    fn ignores_syslog_line_with_trailing_message_text() {
        // try_parse_date recognizes whole-line date *banners*, not a date
        // prefix embedded in an arbitrary longer line - a real syslog
        // message following the timestamp must not be swallowed as if the
        // whole line were just a banner.
        assert!(try_parse_date(
            "RP/0/RSP0/CPU0:Oct 27 20:33:15.826 : %SYS-5-CONFIG_I: Configured from console",
            2026
        )
        .is_none());
    }
}
