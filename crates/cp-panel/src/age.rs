const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

pub fn age_text(now: i64, at: i64) -> String {
    age_in(crate::say::in_english(), now, at)
}

fn age_in(english: bool, now: i64, at: i64) -> String {
    let say = |zh: &'static str, en: &'static str| crate::say::pick_in(english, zh, en);
    let gone = now - at;
    if gone < MINUTE {
        return say("刚刚", "now").into();
    }
    if gone < HOUR {
        return format!("{} {}", gone / MINUTE, say("分钟", "min"));
    }
    if gone < DAY {
        return format!("{} {}", gone / HOUR, say("小时", "h"));
    }
    if gone < 2 * DAY {
        return say("昨天", "yesterday").into();
    }
    if gone < 7 * DAY {
        return format!("{} {}", gone / DAY, say("天", "d"));
    }
    if gone < 30 * DAY {
        return format!("{} {}", gone / (7 * DAY), say("周", "w"));
    }
    if gone < 365 * DAY {
        return format!("{} {}", gone / (30 * DAY), say("个月", "mo"));
    }
    format!("{} {}", gone / (365 * DAY), say("年", "y"))
}

const JUST_NOW: i64 = 10 * MINUTE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    Now,
    Today,
    Yesterday,
    Before,
}

pub fn day_of(at: i64, offset: i64) -> i64 {
    (at + offset * 1_000).div_euclid(DAY)
}

pub fn when_of(now: i64, at: i64, now_offset: i64, at_offset: i64) -> When {
    if now - at < JUST_NOW {
        return When::Now;
    }
    match day_of(now, now_offset) - day_of(at, at_offset) {
        i64::MIN..=0 => When::Today,
        1 => When::Yesterday,
        _ => When::Before,
    }
}

pub fn when_said(when: When, english: bool) -> &'static str {
    let say = |zh: &'static str, en: &'static str| crate::say::pick_in(english, zh, en);
    match when {
        When::Now => say("刚刚", "Now"),
        When::Today => say("今天", "Today"),
        When::Yesterday => say("昨天", "Yesterday"),
        When::Before => say("更早", "Earlier"),
    }
}

pub fn clock_of(at: i64, offset: i64) -> String {
    let minutes = (at + offset * 1_000).div_euclid(MINUTE).rem_euclid(24 * 60);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn civil_of(day: i64) -> (i64, usize, i64) {
    let shifted = day + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * of_year + 2) / 153;
    let day_of_month = of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, usize::try_from(month - 1).unwrap_or(0), day_of_month)
}

pub fn date_of(now: i64, at: i64, now_offset: i64, at_offset: i64, english: bool) -> String {
    let (year, month, day) = civil_of(day_of(at, at_offset));
    let (this_year, _, _) = civil_of(day_of(now, now_offset));
    let same = year == this_year;
    if !english {
        let number = month + 1;
        return if same {
            format!("{number}月{day}日")
        } else {
            format!("{year}年{number}月{day}日")
        };
    }
    let named = MONTHS_EN[month];
    if same {
        format!("{day} {named}")
    } else {
        format!("{day} {named} {year}")
    }
}

pub fn age_in_group(now: i64, at: i64, now_offset: i64, at_offset: i64, english: bool) -> String {
    match when_of(now, at, now_offset, at_offset) {
        When::Now => age_in(english, now, at),
        When::Today | When::Yesterday => clock_of(at, at_offset),
        When::Before => date_of(now, at, now_offset, at_offset, english),
    }
}

#[cfg(test)]
#[path = "age_test.rs"]
mod tests;
