use chrono::{DateTime, Local};

pub fn format_datetime(datetime: DateTime<Local>) -> String {
    datetime
        .format("%Y/%m/%d (%a) %H:%M")
        .to_string()
        .replace("Sun", "日")
        .replace("Mon", "月")
        .replace("Tue", "火")
        .replace("Wed", "水")
        .replace("Thu", "木")
        .replace("Fri", "金")
        .replace("Sat", "土")
}
