use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}
impl Unit {
    pub const ALL: [Self; 4] = [
        Self::Seconds,
        Self::Milliseconds,
        Self::Microseconds,
        Self::Nanoseconds,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Seconds => "秒",
            Self::Milliseconds => "毫秒",
            Self::Microseconds => "微秒",
            Self::Nanoseconds => "纳秒",
        }
    }
    fn scale(self) -> i128 {
        match self {
            Self::Seconds => 1,
            Self::Milliseconds => 1_000,
            Self::Microseconds => 1_000_000,
            Self::Nanoseconds => 1_000_000_000,
        }
    }
}
pub fn offset(text: &str) -> Result<FixedOffset, String> {
    if text == "UTC" || text == "Z" {
        return Ok(FixedOffset::east_opt(0).unwrap());
    }
    let text = text.trim();
    let sign = match text.as_bytes().first() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return Err("时区请输入 +08:00 或 -05:00".into()),
    };
    let (h, m) = text[1..].split_once(':').ok_or("时区格式为 ±HH:MM")?;
    let h: i32 = h.parse().map_err(|_| "无效时区小时")?;
    let m: i32 = m.parse().map_err(|_| "无效时区分钟")?;
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) {
        return Err("时区超出范围".into());
    }
    FixedOffset::east_opt(sign * (h * 3600 + m * 60)).ok_or("时区超出范围".into())
}
pub fn from_timestamp(text: &str, unit: Unit) -> Result<DateTime<Utc>, String> {
    let n: i128 = text.trim().parse().map_err(|_| "请输入整数时间戳")?;
    let scale = unit.scale();
    let seconds = i64::try_from(n.div_euclid(scale)).map_err(|_| "时间戳超出范围")?;
    let nanos = (n.rem_euclid(scale) * (1_000_000_000 / scale)) as u32;
    DateTime::from_timestamp(seconds, nanos).ok_or("时间戳超出范围".into())
}
pub fn timestamp(date: DateTime<Utc>, unit: Unit) -> String {
    (i128::from(date.timestamp()) * unit.scale()
        + i128::from(date.timestamp_subsec_nanos()) / (1_000_000_000 / unit.scale()))
    .to_string()
}
pub fn parse_date(text: &str, zone: FixedOffset) -> Result<DateTime<Utc>, String> {
    let text = text.trim();
    if let Ok(date) =
        DateTime::parse_from_rfc3339(text).or_else(|_| DateTime::parse_from_rfc2822(text))
    {
        return Ok(date.with_timezone(&Utc));
    }
    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y/%m/%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y%m%d%H%M%S",
    ] {
        if let Ok(date) = NaiveDateTime::parse_from_str(text, format) {
            return Ok(zone
                .from_local_datetime(&date)
                .single()
                .ok_or("无效日期")?
                .with_timezone(&Utc));
        }
    }
    for format in ["%Y-%m-%d", "%Y/%m/%d", "%Y%m%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(text, format) {
            return Ok(zone
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .ok_or("无效日期")?
                .with_timezone(&Utc));
        }
    }
    Err("无法识别日期。支持 YYYY-MM-DD HH:mm:ss、斜线日期、紧凑日期、ISO 8601 和 RFC 2822；可含小数秒。".into())
}
pub fn format_custom(
    date: DateTime<Utc>,
    zone: FixedOffset,
    format: &str,
) -> Result<String, String> {
    use std::fmt::Write;
    let items: Vec<_> = chrono::format::StrftimeItems::new(format).collect();
    if items
        .iter()
        .any(|v| matches!(v, chrono::format::Item::Error))
    {
        return Err("无效格式：使用 %Y、%m、%d、%H、%M、%S 等格式符".into());
    }
    let mut result = String::new();
    write!(
        result,
        "{}",
        date.with_timezone(&zone).format_with_items(items.iter())
    )
    .map_err(|_| "日期格式不适用")?;
    Ok(result)
}
// Compatibility entry for cross-tool regression tests.
#[cfg(test)]
pub fn convert(text: &str, reverse: bool) -> Result<String, String> {
    if reverse {
        let date = parse_date(text, offset("+08:00")?)?;
        Ok(format!(
            "秒：{}\n毫秒：{}",
            timestamp(date, Unit::Seconds),
            timestamp(date, Unit::Milliseconds)
        ))
    } else {
        from_timestamp(
            text,
            if text.trim().trim_start_matches('-').len() > 10 {
                Unit::Milliseconds
            } else {
                Unit::Seconds
            },
        )
        .map(|v| v.to_rfc3339())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_units_and_precision() {
        for unit in Unit::ALL {
            for text in ["0", "-1", "1729699200"] {
                assert_eq!(timestamp(from_timestamp(text, unit).unwrap(), unit), text);
            }
        }
        assert!(from_timestamp("999999999999999999999999999", Unit::Seconds).is_err());
    }
    #[test]
    fn date_formats_and_zones() {
        let zone = offset("+08:00").unwrap();
        for text in [
            "1970-01-01 08:00:00",
            "1970/01/01 08:00:00",
            "19700101080000",
            "1970-01-01T00:00:00Z",
            "Thu, 01 Jan 1970 00:00:00 +0000",
        ] {
            assert_eq!(parse_date(text, zone).unwrap().timestamp(), 0);
        }
        assert!(parse_date("2026-02-30", zone).is_err());
        assert!(offset("+08:99").is_err());
        assert_eq!(
            format_custom(
                from_timestamp("0", Unit::Seconds).unwrap(),
                zone,
                "%Y/%m/%d %H:%M:%S"
            )
            .unwrap(),
            "1970/01/01 08:00:00"
        );
        assert!(format_custom(Utc::now(), zone, "%Q").is_err());
    }
}
