//! Timestamp banner parsing: IOS-XR's year-less per-command banner and the
//! year-bearing timestamp printed by "show clock" on IOS/IOS-XE/NX-OS.

use once_cell::sync::Lazy;
use regex::Regex;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: &str = "Mon|Tue|Wed|Thu|Fri|Sat|Sun";

/// IOS-XR auto-printed banner after each command, with no year, e.g.
///   Mon Oct 19 02:45:08.742 UTC
///
/// Not anchored at the start: a queued-command echo can leave stray text
/// stuck to the front of this line (e.g. "showMon Oct 19 ... UTC"), so we
/// only require the banner to occupy the *end* of the line.
static XR_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?:{WEEKDAYS})\s+(?P<mon>{months})\s+(?P<day>\d{{1,2}})\s+\d{{2}}:\d{{2}}:\d{{2}}(?:\.\d+)?\s+\S+\s*$",
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

/// A calendar date in (year, month, day) form, independent of any timezone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl CaptureDate {
    pub fn to_iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn from_iso(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            year: parts[0].parse().ok()?,
            month: parts[1].parse().ok()?,
            day: parts[2].parse().ok()?,
        })
    }
}

/// The result of successfully matching a timestamp banner line.
pub enum ParsedDate {
    /// e.g. "show clock" output, which prints the year explicitly.
    Explicit(CaptureDate, String),
    /// e.g. IOS-XR's auto-printed banner, which has no year; the date here
    /// was built from a caller-supplied best-guess year.
    YearInferred(CaptureDate, String),
}

impl ParsedDate {
    pub fn date(&self) -> CaptureDate {
        match self {
            ParsedDate::Explicit(d, _) | ParsedDate::YearInferred(d, _) => *d,
        }
    }

    /// The exact banner text matched, with any stray leading garbage (see
    /// `XR_DATE_RE`'s docs) already stripped off.
    pub fn banner_text(&self) -> &str {
        match self {
            ParsedDate::Explicit(_, text) | ParsedDate::YearInferred(_, text) => text,
        }
    }

    pub fn had_explicit_year(&self) -> bool {
        matches!(self, ParsedDate::Explicit(..))
    }
}

/// Try to parse a timestamp banner line, using `year_hint` for banners (like
/// IOS-XR's) that don't include a year of their own.
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
    fn iso_roundtrip() {
        let d = CaptureDate {
            year: 2026,
            month: 1,
            day: 5,
        };
        assert_eq!(CaptureDate::from_iso(&d.to_iso()), Some(d));
    }
}
