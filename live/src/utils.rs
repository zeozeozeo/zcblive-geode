use std::ops::RangeInclusive;

pub fn capitalize_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[inline]
pub fn f64_range(range: RangeInclusive<f64>) -> f64 {
    fastrand::f64() * (range.end() - range.start()) + range.start()
}
