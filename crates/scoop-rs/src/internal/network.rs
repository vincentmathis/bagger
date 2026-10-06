#![allow(dead_code)]
use crate::constant::DEFAULT_USER_AGENT;
use curl::easy::Easy;
use std::time::Duration;

/// Fetch the content from a URL as raw bytes plus the HTTP status code.
fn fetch_raw(url: &str, proxy: Option<&str>, headers: &[(&str, &str)]) -> Option<(u32, Vec<u8>)> {
    use curl::easy::List;

    let mut easy = Easy::new();
    easy.get(true).unwrap();
    easy.url(url).unwrap();
    if let Some(proxy) = proxy {
        easy.proxy(proxy).unwrap();
    }
    easy.follow_location(true).unwrap();
    easy.connect_timeout(Duration::from_secs(30)).unwrap();
    easy.useragent(DEFAULT_USER_AGENT).unwrap();

    let mut header_list = List::new();
    for (k, v) in headers {
        header_list.append(&format!("{k}: {v}")).unwrap();
    }
    easy.http_headers(header_list).unwrap();

    let mut content = Vec::new();
    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                content.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().ok()?;
    }

    let code = easy.response_code().unwrap_or(0);
    Some((code, content))
}

/// Fetch the content from a URL as a string, with extra request headers.
///
/// Returns the HTTP status code and the response body on success.
pub fn fetch_url_with_headers(
    url: &str,
    proxy: Option<&str>,
    headers: &[(&str, &str)],
) -> Option<(u32, String)> {
    let (code, raw) = fetch_raw(url, proxy, headers)?;
    String::from_utf8(raw).ok().map(|body| (code, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_get_returns_body() {
        let dir = std::env::temp_dir().join("bagger-probe-net");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello.txt"), b"hi").unwrap();
        let url = format!(
            "file:///{}/hello.txt",
            dir.to_string_lossy().replace('\\', "/")
        );
        let got = fetch_url(&url, None);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(got.as_deref(), Some("hi"));
    }
}

/// Fetch the content from a URL as a string.
pub fn fetch_url(url: &str, proxy: Option<&str>) -> Option<String> {
    fetch_url_with_headers(url, proxy, &[]).map(|(_, body)| body)
}

/// Fetch the content from a URL as raw bytes.
///
/// Unlike [`fetch_url`], this works for non-UTF8 payloads such as
/// archives, making it suitable for hashing downloads.
pub fn fetch_bytes(url: &str, proxy: Option<&str>) -> Option<Vec<u8>> {
    fetch_raw(url, proxy, &[]).map(|(_, body)| body)
}
