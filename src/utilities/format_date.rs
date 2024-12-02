use chrono::NaiveDate;

pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y/%m/%d (%a)")
        .to_string()
        .replace("Sun", "日")
        .replace("Mon", "月")
        .replace("Tue", "火")
        .replace("Wed", "水")
        .replace("Thu", "木")
        .replace("Fri", "金")
        .replace("Sat", "土")
}
