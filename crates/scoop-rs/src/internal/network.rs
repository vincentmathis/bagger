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
