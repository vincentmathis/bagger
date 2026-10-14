pub mod archive;
pub mod aria2;
pub mod dag;
pub mod env;
pub mod fs;
pub mod git;
pub mod network;
pub mod os;
pub mod path;
pub mod ps;

/// Compare two version strings, mirroring the semantics upstream Scoop
/// needs (`Compare-Version` / `app_status`):
///
/// - Dots and dashes both separate components.
/// - All-digit components compare numerically, at any width (length then
///   lexically, so huge build numbers never overflow or fall back to text).
/// - A release beats its prereleases: `2.0.0` > `2.0.0-rc1`, while a
///   numeric continuation wins (`2.0.0.1` > `2.0.0`, `2.0.0.0` == `2.0.0`).
/// - Text components compare run by run (alpha runs case-insensitively,
///   digit runs numerically), so `rc1` < `rc2` and dated nightlies order.
/// - A numeric component beats a text one in the same position
///   (`1.2.0` > `1.2-rc4`).
pub fn compare_versions<S: AsRef<str>>(ver_a: S, ver_b: S) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut a_parts = ver_a.as_ref().split(['.', '-']);
    let mut b_parts = ver_b.as_ref().split(['.', '-']);
    loop {
        match (a_parts.next(), b_parts.next()) {
            (None, None) => return Ordering::Equal,
            // One side runs longer: a numeric continuation wins for its
            // side (all-zero is neutral, keep comparing), while a text
            // continuation marks a prerelease and loses.
            (Some(a), None) => {
                if is_all_digits(a) {
                    if strip_zeros(a) != "0" {
                        return Ordering::Greater;
                    }
                } else {
                    return Ordering::Less;
                }
            }
            (None, Some(b)) => {
                if is_all_digits(b) {
                    if strip_zeros(b) != "0" {
                        return Ordering::Less;
                    }
                } else {
                    return Ordering::Greater;
                }
            }
            (Some(a), Some(b)) => match compare_part(a, b) {
                Ordering::Equal => continue,
                ord => return ord,
            },
        }
    }
}

/// Compare one dot/dash-separated component, tokenized into alternating
/// digit and non-digit runs.
fn compare_part(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut a_runs = runs(a);
    let mut b_runs = runs(b);
    loop {
        match (a_runs.next(), b_runs.next()) {
            (None, None) => return Ordering::Equal,
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (Some(x), Some(y)) => match compare_run(&x, &y) {
                Ordering::Equal => continue,
                ord => return ord,
            },
        }
    }
}

#[derive(PartialEq)]
enum Run {
    Num(String),
    Text(String),
}

/// Split `1a02b` into digit/non-digit runs; digit runs shed leading zeros
/// (empty means zero).
fn runs(s: &str) -> impl Iterator<Item = Run> + '_ {
    let mut rest = s;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let numeric = rest.as_bytes()[0].is_ascii_digit();
        let len = rest
            .bytes()
            .take_while(|b| b.is_ascii_digit() == numeric)
            .count();
        let (head, tail) = rest.split_at(len);
        rest = tail;
        if numeric {
            Some(Run::Num(strip_zeros(head).to_owned()))
        } else {
            Some(Run::Text(head.to_ascii_lowercase()))
        }
    })
}

fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn strip_zeros(s: &str) -> &str {
    let stripped = s.trim_start_matches('0');
    if stripped.is_empty() {
        "0"
    } else {
        stripped
    }
}

fn compare_run(a: &Run, b: &Run) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Run::Num(x), Run::Num(y)) => compare_numbers(x, y),
        (Run::Num(_), Run::Text(_)) => Ordering::Greater,
        (Run::Text(_), Run::Num(_)) => Ordering::Less,
        (Run::Text(x), Run::Text(y)) => x.cmp(y),
    }
}

/// Numeric order for digit strings of any width.
fn compare_numbers(a: &str, b: &str) -> std::cmp::Ordering {
    // Both already zero-stripped: longer means larger.
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering::*;

    #[test]
    fn version_ordering_covers_prereleases() {
        // Plain numerics, any width (no u32 overflow fallback to text).
        assert_eq!(compare_versions("1.2.3", "1.2.3"), Equal);
        assert_eq!(compare_versions("1.2.3", "1.2.4"), Less);
        assert_eq!(compare_versions("1.10.0", "1.9.9"), Greater);
        assert_eq!(compare_versions("1791766981", "1791766980"), Greater);
        assert_eq!(
            compare_versions("99999999999999999999", "99999999999999999998"),
            Greater
        );
        // Release beats its prereleases; numeric continuations win.
        assert_eq!(compare_versions("2.0.0", "2.0.0-rc1"), Greater);
        assert_eq!(compare_versions("2.0.0-rc1", "2.0.0"), Less);
        assert_eq!(compare_versions("2.0.0.1", "2.0.0"), Greater);
        assert_eq!(compare_versions("2.0.0.0", "2.0.0"), Equal);
        assert_eq!(compare_versions("1.2.0", "1.2-rc4"), Greater);
        // Text orders run by run.
        assert_eq!(compare_versions("1.0-rc1", "1.0-rc2"), Less);
        assert_eq!(compare_versions("1.0-beta", "1.0-rc1"), Less);
        assert_eq!(
            compare_versions("nightly-20261012", "nightly-20261013"),
            Less
        );
        assert_eq!(compare_versions("1.0-RC1", "1.0-rc1"), Equal);
    }
}
