#![allow(dead_code)]
use crate::constant::DEFAULT_USER_AGENT;
use curl::easy::Easy;
use std::time::Duration;

pub fn get_content_length(url: &str, proxy: Option<&str>) -> Option<f64> {
    let mut easy = Easy::new();
    easy.get(true).unwrap();
    easy.url(url).unwrap();
    if let Some(proxy) = proxy {
        easy.proxy(proxy).unwrap();
    }
    easy.nobody(true).unwrap();
    easy.follow_location(true).unwrap();
    easy.connect_timeout(Duration::from_secs(30)).unwrap();
    println!("get_content_length of: {}", url);
    easy.perform().unwrap();
    easy.content_length_download().ok()
}

/// Fetch the content from a URL as a string, with extra request headers.
///
/// Returns the HTTP status code and the response body on success.
pub fn fetch_url_with_headers(
    url: &str,
    proxy: Option<&str>,
    headers: &[(&str, &str)],
) -> Option<(u32, String)> {
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
    String::from_utf8(content).ok().map(|body| (code, body))
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
    let mut easy = Easy::new();
    easy.get(true).unwrap();
    easy.url(url).unwrap();
    if let Some(proxy) = proxy {
        easy.proxy(proxy).unwrap();
    }
    easy.follow_location(true).unwrap();
    easy.connect_timeout(Duration::from_secs(30)).unwrap();
    easy.useragent(DEFAULT_USER_AGENT).unwrap();

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

    String::from_utf8(content).ok()
}
