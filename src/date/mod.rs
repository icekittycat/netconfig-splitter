//! Timestamp banner/line parsing.

pub mod cisco;

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

/// The result of successfully matching a timestamp line.
pub enum ParsedDate {
    /// The source line printed the year explicitly (e.g. Cisco's
    /// "show clock" output, or any Juniper timestamp).
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

    /// The exact banner text matched, with any stray leading garbage
    /// already stripped off.
    pub fn banner_text(&self) -> &str {
        match self {
            ParsedDate::Explicit(_, text) | ParsedDate::YearInferred(_, text) => text,
        }
    }

    pub fn had_explicit_year(&self) -> bool {
        matches!(self, ParsedDate::Explicit(..))
    }
}

/// Try to parse a timestamp line, using `year_hint` for patterns (like
/// IOS-XR's) that don't include a year of their own.
pub fn try_parse_date(line: &str, year_hint: i32) -> Option<ParsedDate> {
    cisco::try_parse_date(line, year_hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_to_cisco_format() {
        let parsed = try_parse_date("Mon Oct 19 02:45:08.742 UTC", 2026).unwrap();
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
    fn iso_roundtrip() {
        let d = CaptureDate {
            year: 2026,
            month: 1,
            day: 5,
        };
        assert_eq!(CaptureDate::from_iso(&d.to_iso()), Some(d));
    }
}
