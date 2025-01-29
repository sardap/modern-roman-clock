mod utils;

use wasm_bindgen::prelude::*;

use std::{convert::TryFrom, time::Duration};

use chrono::{DateTime, Datelike, Days, TimeDelta, TimeZone, Timelike};
use num_ordinal::Ordinal;
use sunrise::sunrise_sunset;

include!(concat!(env!("OUT_DIR"), "/year_owner.rs"));

#[wasm_bindgen]
pub struct RomanTime {
    day: u32,
    month: u32,
    year: i32,
    hour: i32,
    hour_progress: f64,
    daylight_length: chrono::Duration,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl RomanTime {
    #[wasm_bindgen(constructor)]
    pub fn new_js(time: js_sys::Date, tz: String, lat: f64, lng: f64) -> Self {
        use chrono::prelude::*;

        use chrono_tz::Tz;

        let tz: Tz = tz.parse().unwrap();

        let time = NaiveDate::from_ymd_opt(
            time.get_full_year() as i32,
            time.get_month() + 1,
            time.get_date(),
        )
        .unwrap()
        .and_hms_opt(time.get_hours(), time.get_minutes(), time.get_seconds())
        .unwrap();

        let time = tz.from_local_datetime(&time).unwrap();
        RomanTime::new(time, lat, lng)
    }
}

impl RomanTime {
    pub fn new<Tz: TimeZone>(time: DateTime<Tz>, lat: f64, lng: f64) -> Self {
        let (sunrise, sunset) = sunrise_sunset(lat, lng, time.year(), time.month(), time.day());
        let timezone = time.timezone();
        let time = time.naive_local();
        let midnight_today = time
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        let sunrise = chrono::Utc
            .timestamp_opt(sunrise, 0)
            .unwrap()
            .with_timezone(&timezone)
            .naive_local();
        let sunset = chrono::Utc
            .timestamp_opt(sunset, 0)
            .unwrap()
            .with_timezone(&timezone)
            .naive_local();

        let (hour, hour_progress) = if time < sunrise {
            // get time until sunrise
            let time_since_midnight = time - midnight_today;
            let morning_night_length = sunrise - midnight_today;
            let (hour, fract) = hour_breakdown(time_since_midnight, morning_night_length, 6);
            (hour + 6 + 12, fract)
        } else if time > sunset {
            // get time since sunset
            let midnight_tomorrow = midnight_today + Duration::from_secs(24 * 60 * 60);
            let time_since_sunset = time - sunset;
            let evening_night_length = midnight_tomorrow - sunset;
            let (hour, fract) = hour_breakdown(time_since_sunset, evening_night_length, 6);
            (hour + 12, fract)
        } else {
            // Daytime
            let time_since_sunrise = time - sunrise;
            let daylight_length = sunset - sunrise;
            let (hour, fract) = hour_breakdown(time_since_sunrise, daylight_length, 12);
            (hour, fract)
        };

        let roman_date = if time < sunrise {
            time.checked_sub_days(Days::new(1)).unwrap()
        } else {
            time
        };

        // Here get daylight hour length and night hour length

        return RomanTime {
            day: roman_date.day(),
            month: roman_date.month(),
            year: roman_date.year(),
            hour,
            hour_progress,
            daylight_length: chrono::Duration::seconds((sunset - sunrise).num_seconds()),
        };
    }

    pub fn daylight_length(&self) -> chrono::Duration {
        self.daylight_length
    }

    pub fn night_length(&self) -> chrono::Duration {
        chrono::Duration::seconds(24 * 60 * 60) - self.daylight_length
    }
}

const FULL_MONTHS: [u32; 7] = [1, 3, 5, 7, 8, 10, 12];

#[wasm_bindgen]
impl RomanTime {
    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn month(&self) -> u32 {
        self.month
    }

    pub fn day(&self) -> u32 {
        self.day
    }

    pub fn hour(&self) -> i32 {
        self.hour
    }

    pub fn hour_progress(&self) -> f64 {
        self.hour_progress
    }

    #[cfg(target_arch = "wasm32")]
    pub fn daylight_length_seconds(&self) -> i64 {
        self.daylight_length().num_seconds()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn night_length_seconds(&self) -> i64 {
        self.night_length().num_seconds()
    }

    pub fn year_string(&self, country_iso_3166: &str) -> String {
        let owners: &[YearOwner] = match get_owners_for_country(country_iso_3166) {
            Some(owners) => owners,
            None => return self.year().to_string(),
        };

        for owner in owners.iter() {
            for (i, years) in owner.years.iter().enumerate() {
                if self.year >= years.0 && self.year <= years.1 {
                    let mut year_count = 0;
                    for j in 0..i {
                        year_count += (owner.years[j].1 - owner.years[j].0) + 1;
                    }

                    year_count += self.year - years.0;

                    year_count += 1;

                    return format!(
                        "{} year of {}",
                        num_ordinal::Osize::from1(year_count as usize),
                        owner.owner
                    );
                }
            }
        }

        return self.year().to_string();
    }

    pub fn date_string(&self) -> String {
        let is_full_month = FULL_MONTHS.iter().any(|&x| x == self.month);

        let month = chrono::Month::try_from((self.month) as u8).unwrap();
        let month_string = month.name().to_string();

        if self.day == 1  {
            return "Kalends of ".to_string() + &month_string;
        }

        let nones_date = if is_full_month { 7 } else { 5 };
        if self.day <= nones_date {
            let remaining = nones_date - self.day;
            if remaining == 0 {
                return format!("Nones of {}", month_string);
            } else if remaining == 1 {
                return format!("day before the Nones of {}", month_string);
            }
            return format!(
                "{} day before the Nones of {}",
                num_ordinal::Osize::from1(remaining as usize + 1),
                month_string
            );
        }

        let ides_date = if is_full_month { 15 } else { 13 };
        if self.day <= ides_date {
            let remaining = ides_date - self.day;
            if remaining == 0 {
                return format!("Ides of {}", month_string);
            } else if remaining == 1 {
                return format!("day before the Ides of {}", month_string);
            }
            return format!(
                "{} day before the Ides of {}",
                num_ordinal::Osize::from1(remaining as usize + 1),
                month_string
            );
        }

        // Confusingly, once the ides pass we talk about the next month
        let next_month = (self.month + 1) % 12;
        let next_month_name = chrono::Month::try_from(next_month as u8).unwrap().name();
        let leap_year = self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0);
        let days_in_month = match self.month {
            2 => {
                if leap_year {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };

        let remaining = days_in_month - self.day;
        if remaining == 0 {
            return format!("day before the Kalends of {}", next_month_name);
        }

        format!(
            "{} day before the Kalends of {}",
            num_ordinal::Osize::from1(remaining as usize + 2),
            next_month_name
        )
    }

    pub fn hour_string(&self) -> String {
        let hour = self.hour() + 1;

        let progress_part = if self.hour_progress <= 0.25 {
            "less than a quarter"
        } else if self.hour_progress <= 0.5 {
            "less than half"
        } else if self.hour_progress <= 0.75 {
            "less than three quarters"
        } else {
            "more than three quarters"
        };

        let night_time = hour >= 13;
        let hour = if hour > 12 { hour - 12 } else { hour };

        format!(
            "{} of the {} {} hour",
            progress_part,
            num_ordinal::Osize::from1(hour as usize),
            if night_time { "night" } else { "daylight" }
        )
    }

    pub fn to_string(&self, country_iso_3166: &str) -> String {
        format!(
            "{} of {} {}",
            self.hour_string(),
            self.date_string(),
            self.year_string(country_iso_3166),
        )
    }
}

fn hour_breakdown(time_since: TimeDelta, total_length: TimeDelta, hour_amount: u8) -> (i32, f64) {
    let hour_length = total_length.num_seconds() as f64 / hour_amount as f64;
    let hour = time_since.num_seconds() as f64 / hour_length;
    (hour.floor() as i32, hour.fract())
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    const BROKEN_HILL_LAT: f64 = -31.9596256;
    const BROKEN_HILL_LNG: f64 = 141.4575006;

    #[test]
    fn broken_hill_before_sunrise() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 27, 6, 20, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.day(), 26);
        assert_eq!(roman_time.hour(), 23);
        assert!(roman_time.hour_progress() > 0.9);
    }

    #[test]
    fn broken_hill_just_after_sunrise() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 27, 6, 25, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.day(), 27);
        assert_eq!(roman_time.hour(), 0);
        assert!(roman_time.hour_progress() < 0.1);
    }

    #[test]
    fn broken_hill_just_after_sunset() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 27, 20, 10, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.day(), 27);
        assert_eq!(roman_time.hour(), 12);
        assert!(roman_time.hour_progress() < 0.1);
    }

    #[test]
    fn broken_hill_solar_noon() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 27, 13, 16, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.hour(), 5);
        assert!(roman_time.hour_progress() > 0.9);
    }

    #[test]
    fn before_sunrise_first_of_month() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 01, 1, 0, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.day(), 31);
        assert_eq!(roman_time.month(), 12);
        assert_eq!(roman_time.year(), 2024);
    }

    #[test]
    fn year_string() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 01, 27, 12, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.year_string("AU"), "5th year of Paul Keating");

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1941, 02, 01, 1, 0, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.year_string("AU"),
            "second year of Robert Menzies"
        );

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1951, 02, 01, 1, 0, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.year_string("AU"), "4th year of Robert Menzies");
        assert_eq!(roman_time.year_string("US"), "6th year of Harry S Truman");
    }

    #[test]
    fn date_string() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 02, 1, 3, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.date_string(),
            "day before the Kalends of February"
        );

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 01, 27, 12, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.date_string(),
            "6th day before the Kalends of February"
        );

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 03, 3, 12, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.date_string(),
            "5th day before the Nones of March"
        );

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 03, 7, 7, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.date_string(), "Nones of March");

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 03, 15, 7, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.date_string(), "Ides of March");

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 03, 14, 7, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.date_string(), "day before the Ides of March");

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 03, 13, 7, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.date_string(),
            "third day before the Ides of March"
        );
    }

    #[test]
    fn hour_string() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 28, 22, 48, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.hour_string(),
            "less than a quarter of the 5th night hour"
        );

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(1996, 01, 27, 12, 45, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.hour_string(),
            "less than three quarters of the 6th daylight hour"
        );

        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 28, 6, 48, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(
            roman_time.hour_string(),
            "less than half of the first daylight hour"
        );
    }

    #[test]
    fn to_string() {
        for (hour, minute, second) in (0..24)
            .cartesian_product(0..60)
            .cartesian_product(0..60)
            .map(|((h, m), s)| (h, m, s))
        {
            let time = chrono_tz::Australia::Broken_Hill
                .with_ymd_and_hms(2025, 01, 28, hour, minute, second)
                .unwrap();
            let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
            assert!(roman_time.to_string("AU").len() > 0);
        }
    }

    #[test]
    fn daylight_length() {
        let time = chrono_tz::Australia::Broken_Hill
            .with_ymd_and_hms(2025, 01, 28, 6, 48, 0)
            .unwrap();
        let roman_time = RomanTime::new(time, BROKEN_HILL_LAT, BROKEN_HILL_LNG);
        assert_eq!(roman_time.daylight_length().num_seconds() / 12 / 60, 68);
    }
}
