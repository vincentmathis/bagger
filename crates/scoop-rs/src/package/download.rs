use curl::easy::{Easy, List};
use curl::multi::Multi;
use flume::Sender;
use once_cell::unsync::OnceCell;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    time::Duration,
};
use tracing::debug;

use crate::constant::DEFAULT_USER_AGENT;
use crate::{error::Fallible, internal, Event, Session};

use super::Package;

/// Download size information.
#[derive(Clone, Copy)]
pub struct DownloadSize {
    /// Total size to download.
    pub total: u64,

    /// Whether the total size is estimated.
    pub estimated: bool,
}

/// A set of packages to download.
pub struct PackageSet<'a> {
    /// Associated scoop_rs session.
    session: &'a Session,

    /// Packages with intent to download.
    pub packages: &'a [&'a Package],

    /// Multi handle for curl.
    multi: Multi,

    caches: OnceCell<HashMap<String, PackageCache<'a>>>,

    /// Whether to reuse cached files.
    reuse_cache: bool,
}

/// Stores download information of a file.
struct FileDownloadInfo<'a> {
    /// Download URL.
    url: &'a str,

    /// Local cached file size.
    local_size: u64,

    /// Remote file size.
    remote_size: u64,

    /// Whether the remote file size is estimated.
    estimated: bool,
}

/// Possible cache state of a package.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CacheMaybeValid {
    /// All files are cached and valid.
    Full,

    /// Some files are cached and valid.
    Partial,

    /// No valid cache.
    None,
}

/// Local cache information of a package.
struct PackageCache<'a> {
    /// Associated package.
    package: &'a Package,

    /// Whether the cache is valid.
    valid: CacheMaybeValid,

    /// Inner details of the package cache.
    ///
    /// Since a package may have multiple files to download, the inner hashmap
    /// stores the download information of each file.
    inner: HashMap<String, FileDownloadInfo<'a>>,
}

impl PackageCache<'_> {
    fn update_valid_state(&mut self) {
        let mut cnt = 0;
        for cache in self.inner.values() {
            // A file only counts as valid when its remote size is known and
            // matches the local size. Unknown remote sizes (servers omitting
            // Content-Length, non-HTTP URLs) must never validate a missing
            // (or any) local file, otherwise downloads would be skipped and
            // the integrity check would fail on absent files.
            if cache.remote_size > 0 && cache.local_size == cache.remote_size {
                cnt += 1;
            }
        }

        if cnt == self.inner.len() {
            self.valid = CacheMaybeValid::Full;
        } else if cnt > 0 {
            self.valid = CacheMaybeValid::Partial;
        } else {
            self.valid = CacheMaybeValid::None;
        }
    }
}

impl<'a> PackageSet<'a> {
    pub fn new(
        session: &'a Session,
        packages: &'a [&Package],
        reuse_cache: bool,
    ) -> Fallible<PackageSet<'a>> {
        let mut multi = Multi::new();

        // TODO: configurable max connections
        multi.set_max_total_connections(6)?;
        multi.set_max_host_connections(4)?;
        multi.pipelining(false, true)?;

        Ok(PackageSet {
            session,
            packages,
            multi,
            caches: OnceCell::new(),
            reuse_cache,
        })
    }

    fn load_cache(&self) {
        if self.caches.get().is_some() {
            return;
        }

        let config = self.session.config();
        let cache_root = config.cache_path();

        let mut caches = HashMap::new();

        for &pkg in self.packages.iter() {
            // if the package is upgradable, use the upgradable reference instead
            let pkg = pkg.upgradable().unwrap_or(pkg);

            let urls = pkg.download_urls();
            let filenames = pkg.download_filenames();

            let mut pacakge_cache = PackageCache {
                package: pkg,
                valid: CacheMaybeValid::None,
                inner: HashMap::new(),
            };

            let mut file_cached_count = 0;
            for (url, filename) in urls.iter().zip(filenames.iter()) {
                let remote_size = 0u64;
                let mut local_size = 0u64;

                if self.reuse_cache {
                    if let Ok(file) = File::open(cache_root.join(filename)) {
                        if let Ok(metadata) = file.metadata() {
                            local_size = metadata.len();
                            file_cached_count += 1;
                        }
                    }
                }

                let dlinfo = FileDownloadInfo {
                    url,
                    local_size,
                    remote_size,
                    estimated: false,
                };

                pacakge_cache.inner.insert(filename.to_owned(), dlinfo);
            }

            if self.reuse_cache {
                if file_cached_count == urls.len() {
                    pacakge_cache.valid = CacheMaybeValid::Full;
                } else if file_cached_count > 0 {
                    pacakge_cache.valid = CacheMaybeValid::Partial;
                }
            }

            caches.insert(pkg.ident(), pacakge_cache);
        }

        let _ = self.caches.set(caches);
    }

    /// Download packages.
    pub fn download(&mut self) -> Fallible<()> {
        if self.caches.get().is_none() {
            self.load_cache();
        }

        // Delegate to aria2c when enabled and available.
        if self.session.config().aria2_enabled() {
            if internal::aria2::is_available() {
                match self.download_via_aria2() {
                    Ok(()) => return Ok(()),
                    Err(crate::Error::Aria2 { code, .. })
                        if self.session.config().aria2_fallback_enabled() =>
                    {
                        // Upstream `ARIA2-FALLBACK-ENABLED` (default on):
                        // retry through the default downloader instead of
                        // aborting the whole transaction.
                        eprintln!(
                            "warning: download failed! (Error {code}) {}",
                            internal::aria2::exit_code_message(code)
                        );
                        eprintln!("warning: fallback to default downloader...");
                        self.remove_aria2_control_files();
                    }
                    Err(e) => return Err(e),
                }
            } else if self.session.config().aria2_warning_enabled() {
                eprintln!(
                    "warning: aria2 is enabled but no 'aria2c' binary was found on PATH; falling back to curl"
                );
            }
        }

        self.download_via_curl()
    }

    /// Remove stale aria2c control (`.aria2`) files from the cache dir so a
    /// curl retry starts clean (upstream removes `$source.aria2*`).
    fn remove_aria2_control_files(&self) {
        let cache_root = self.session.config().cache_path().to_owned();
        let entries = std::fs::read_dir(&cache_root);
        for entry in entries.into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "aria2") {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    /// Download pending files with the aria2c external downloader.
    fn download_via_aria2(&mut self) -> Fallible<()> {
        let config = self.session.config();
        let cache_root = config.cache_path().to_owned();
        let proxy = config.proxy().map(|s| s.to_owned());
        let split = config.aria2_split();
        let max_connection_per_server = config.aria2_max_connection_per_server();
        let min_split_size = config.aria2_min_split_size().to_owned();
        let retry_wait = config.aria2_retry_wait();
        let extra_options = config.aria2_options().map(|s| s.to_owned());
        drop(config);

        let user_agent = self
            .session
            .user_agent
            .get()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT)
            .to_owned();

        let aria2_opts = internal::aria2::DownloadOptions {
            user_agent,
            cookie: String::new(),
            proxy: proxy.clone(),
            split,
            max_connection_per_server,
            min_split_size,
            retry_wait,
            extra_options,
        };

        // ensure cache dir exists
        internal::fs::ensure_dir(&cache_root)?;

        let package_caches = self.caches.get_mut().unwrap();
        let mut filepaths = vec![];

        for cache in package_caches.values() {
            // skip download if all files are cached and valid
            if self.reuse_cache && cache.valid == CacheMaybeValid::Full {
                continue;
            }

            let cookie = cache.package.cookie().unwrap_or_default();
            let cookie = cookie
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");

            for (filename, dlinfo) in cache.inner.iter() {
                if self.reuse_cache
                    && dlinfo.local_size > 0
                    && dlinfo.local_size == dlinfo.remote_size
                {
                    continue;
                }

                let tmp = cache_root.join(format!("{}.download", filename));
                let path = cache_root.join(filename);

                // remove possible existing files
                let _ = std::fs::remove_file(&path);
                let _ = std::fs::remove_file(&tmp);

                let opts = internal::aria2::DownloadOptions {
                    cookie: cookie.clone(),
                    ..aria2_opts.clone()
                };
                // Special hosts (FossHub handshake, SourceForge reshape)
                // resolve to the real file URL before fetching.
                let fetch_url = crate::operation::resolve_special_url(dlinfo.url, proxy.as_deref());
                internal::aria2::download_file(&fetch_url, &tmp, &opts)?;

                filepaths.push((tmp, path));
            }
        }

        for (tmp, path) in filepaths.iter() {
            std::fs::rename(tmp, path)?;
        }

        Ok(())
    }

    /// Download pending files with the built-in curl backend.
    fn download_via_curl(&mut self) -> Fallible<()> {
        if self.caches.get().is_none() {
            self.load_cache();
        }

        let config = self.session.config();
        let cache_root = config.cache_path();
        let proxy = config.proxy();
        let user_agent = self
            .session
            .user_agent
            .get()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT);

        let mut handles = HashMap::new();
        let mut token_ctx = HashMap::new();
        let package_caches = self.caches.get_mut().unwrap();

        // map download tmp files to their final names
        let mut filepaths = vec![];

        // ensure cache dir exists
        internal::fs::ensure_dir(&cache_root)?;

        for (pidx, (_, cache)) in package_caches.iter().enumerate() {
            // skip download if all files are cached and valid
            if self.reuse_cache && cache.valid == CacheMaybeValid::Full {
                continue;
            }

            let cookie = cache.package.cookie().unwrap_or_default();

            for (uidx, (filename, dlinfo)) in cache.inner.iter().enumerate() {
                if self.reuse_cache
                    && dlinfo.local_size > 0
                    && dlinfo.local_size == dlinfo.remote_size
                {
                    continue;
                }

                let mut easy = Easy::new();
                easy.get(true)?;
                // Special hosts (FossHub handshake, SourceForge reshape)
                // resolve to the real file URL before fetching. Cache keys
                // and staged names intentionally keep the original URL.
                let fetch_url = crate::operation::resolve_special_url(dlinfo.url, proxy);
                easy.url(&fetch_url)?;
                easy.follow_location(true)?;
                easy.useragent(user_agent)?;
                easy.fail_on_error(true)?;
                if let Some(proxy) = proxy {
                    easy.proxy(proxy)?;
                }
                set_cookie(
                    &mut easy,
                    &cookie,
                    &crate::operation::headers_for_url(self.session, dlinfo.url),
                )?;
                if let Some(referer) = referer_for_url(dlinfo.url) {
                    easy.referer(&referer)?;
                }

                if let Some(tx) = self.session.emitter() {
                    let ident = cache.package.ident();
                    let url = dlinfo.url.to_owned();
                    let fname = filename.to_owned();
                    easy.progress(true)?;
                    easy.progress_function(move |dltotal, dlnow, _, _| {
                        progress(
                            tx.clone(),
                            ident.to_owned(),
                            url.to_owned(),
                            fname.to_owned(),
                            dltotal,
                            dlnow,
                        )
                    })?;
                }

                let path = cache_root.join(filename);
                let tmp = cache_root.join(format!("{}.download", filename));

                // remove possible existing files
                let _ = std::fs::remove_file(&path);
                let _ = std::fs::remove_file(&tmp);

                filepaths.push((tmp.clone(), path.clone()));

                // TODO: Fragmented download support could be added to improve
                // download speed.
                let mut file = OpenOptions::new().create(true).append(true).open(&tmp)?;
                easy.write_function(move |data| {
                    file.write_all(data).unwrap();
                    Ok(data.len())
                })?;

                let mut easyhandle = self.multi.add(easy)?;
                let token = pidx * 100 + uidx;
                let _ = easyhandle.set_token(token);
                handles.insert(token, easyhandle);

                token_ctx.insert(token, (cache.package.ident(), filename.to_owned()));
            }
        }

        let mut alive = true;
        while alive {
            alive = self.multi.perform()? > 0;

            let mut handle_err = None;

            self.multi.messages(|message| {
                let token = message.token().expect("failed to get token");
                let handle = handles.get_mut(&token).expect("failed to get handle");

                // catch and propagate curl error
                if let Some(Err(e)) = message.result_for(handle) {
                    handle_err = Some(e);
                }
            });

            if let Some(err) = handle_err {
                return Err(err.into());
            }

            if alive {
                self.multi.wait(&mut [], Duration::from_secs(5))?;
            }
        }

        for (tmp, path) in filepaths.iter() {
            std::fs::rename(tmp, path)?;
        }

        Ok(())
    }

    /// Calculate download size.
    ///
    /// This function is actually a pre-download process, which will try to
    /// fetch the remote file size of each package file.
    pub fn calculate_download_size(&mut self) -> Fallible<DownloadSize> {
        if self.caches.get().is_none() {
            self.load_cache();
        }

        let config = self.session.config();
        let proxy = config.proxy();
        let user_agent = self
            .session
            .user_agent
            .get()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT);

        let mut handles = HashMap::new();
        let mut token_ctx = HashMap::new();
        let package_caches = self.caches.get_mut().unwrap();

        for (pidx, &pkg) in self.packages.iter().enumerate() {
            // if the package is upgradable, use the upgradable reference instead
            let pkg = pkg.upgradable().unwrap_or(pkg);

            let urls = pkg.download_urls();
            let filenames = pkg.download_filenames();
            let cookie = pkg.cookie().unwrap_or_default();

            for (uidx, (url, filename)) in urls.iter().zip(filenames.iter()).enumerate() {
                let mut easy = Easy::new();
                easy.get(true)?;
                easy.url(url)?;
                easy.follow_location(true)?;
                easy.nobody(true)?;
                easy.useragent(user_agent)?;
                if let Some(proxy) = proxy {
                    easy.proxy(proxy)?;
                }
                // Size probes go without auth headers (upstream only sends
                // them on the actual download).
                set_cookie(&mut easy, &cookie, &[])?;

                let mut easyhandle = self.multi.add(easy)?;
                let token = pidx * 100 + uidx;
                let _ = easyhandle.set_token(token);
                handles.insert(token, easyhandle);

                token_ctx.insert(token, (pkg.ident(), url.to_string(), filename.to_owned()));
            }
        }

        let mut total = 0;
        let mut estimated = false;

        let mut alive = true;
        while alive {
            alive = self.multi.perform()? > 0;

            let mut handle_err = None;

            self.multi.messages(|message| {
                let token = message.token().expect("failed to get token");
                let handle = handles.get_mut(&token).expect("failed to get handle");

                if let Some(handle_ret) = message.result_for(handle) {
                    match handle_ret {
                        Err(e) => handle_err = Some(e),
                        Ok(_) => {
                            let (ident, url, filename) = token_ctx.get(&token).unwrap();
                            let package_cache = package_caches.get_mut(ident).unwrap();
                            let info = package_cache
                                .inner
                                .get_mut(filename)
                                .expect("failed to get cache info");

                            if let Ok(code) = handle.response_code() {
                                let mut content_length = 0u64;
                                if code == 200 {
                                    content_length =
                                        handle.content_length_download().unwrap_or(0f64) as u64;
                                    info.remote_size = content_length;
                                    if content_length != info.local_size {
                                        total += content_length;
                                    }
                                } else {
                                    debug!("code: {}, ident: {}, url: {}", code, ident, url)
                                }

                                if content_length == 0 {
                                    info.estimated = true;
                                    estimated = true;
                                }

                                package_cache.update_valid_state();
                            } else {
                                debug!("failed to get response code for {}", url);
                            }
                        }
                    }
                }
            });

            if let Some(err) = handle_err {
                return Err(err.into());
            }

            if alive {
                self.multi.wait(&mut [], Duration::from_secs(5))?;
            }
        }

        Ok(DownloadSize { total, estimated })
    }
}

/// Upstream sets `Referer` to the file's directory, except for
/// sourceforge/portableapps hosts. Non-HTTP(S) URLs get none.
fn referer_for_url(url: &str) -> Option<String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains("sourceforge.net") || lower.contains("portableapps.com") {
        return None;
    }
    let stripped = url.split('#').next().unwrap_or(url);
    Some(stripped.rsplit_once('/')?.0.to_owned())
}

#[cfg(test)]
mod referer_tests {
    use super::referer_for_url;

    #[test]
    fn referer_points_at_file_directory() {
        assert_eq!(
            referer_for_url("https://example.com/dir/app-1.0.zip"),
            Some("https://example.com/dir".to_owned())
        );
        assert_eq!(
            referer_for_url("https://example.com/dir/app.zip?dl=1#/dl.7z"),
            Some("https://example.com/dir".to_owned())
        );
        assert_eq!(referer_for_url("file:///C:/x/y.cmd"), None);
        assert_eq!(
            referer_for_url("https://downloads.sourceforge.net/project/a/b.exe"),
            None
        );
        assert_eq!(referer_for_url("https://portableapps.com/apps/a.exe"), None);
    }
}

fn set_cookie(
    easy: &mut Easy,
    cookie: &[(&str, &str)],
    extra_headers: &[(String, String)],
) -> Fallible<()> {
    if cookie.is_empty() && extra_headers.is_empty() {
        return Ok(());
    }
    let mut list = List::new();
    if !cookie.is_empty() {
        let mut header_cookie = String::from("Cookie: ");
        header_cookie.push_str(
            &cookie
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; "),
        );
        list.append(&header_cookie)?;
    }
    // `private_hosts` entries whose `match` regex hits the URL contribute
    // extra headers (upstream `Invoke-Download` semantics).
    for (name, value) in extra_headers {
        list.append(&format!("{name}: {value}"))?;
    }
    easy.http_headers(list)?;

    Ok(())
}

/// Progress context for package download.
#[derive(Clone, Debug)]
pub struct PackageDownloadProgressContext {
    /// Package identifier.
    pub ident: String,

    /// Download URL.
    pub url: String,

    /// Download filename.
    pub filename: String,

    /// Total bytes to download.
    pub dltotal: u64,

    /// Downloaded bytes.
    pub dlnow: u64,
}

/// Report package download progress.
fn progress(
    tx: Sender<Event>,
    ident: String,
    url: String,
    filename: String,
    dltotal: f64,
    dlnow: f64,
) -> bool {
    let ctx = PackageDownloadProgressContext {
        ident,
        url,
        filename,
        dltotal: dltotal as u64,
        dlnow: dlnow as u64,
    };

    // TODO: progress threshold
    // it's not that efficient to send progress event to report every progress
    // change.
    tx.send(Event::PackageDownloadProgress(ctx)).is_ok()
}

// #[derive(Debug)]
// struct ChunkedRange {
//     pub offset: u64,
//     pub length: u64,
//     pub data: [u8; 4096],
// }

// fn download_packages<F>(
//     session: &Session,
//     packages: &Vec<Package>,
//     no_cache: bool,
//     callback: F,
// ) -> Fallible<()>
// where
//     F: FnMut(DownloadProgressContext) + Send + 'static,
// {
//     let callback = Arc::new(Mutex::new(callback));
//     let mut client = AgentBuilder::new();
//     let user_agent = match session.user_agent.filled() {
//         true => session.user_agent.borrow().unwrap().to_owned(),
//         false => constant::DEFAULT_USER_AGENT.to_string(),
//     };
//     client = client.user_agent(&user_agent);
//     let config = session.get_config();
//     if config.proxy().is_some() {
//         let proxy = config.proxy().unwrap();
//         let proxy = ureq::Proxy::new(proxy)?;
//         client = client.proxy(proxy);
//     }
//     let client = client.build();

//     for package in packages {
//         let urls = package.manifest.url();
//         let cookie = package.manifest.cookie();

//         let file_count = urls.len();

//         for (index, url) in urls.into_iter().enumerate() {
//             let index = index + 1;
//             let client = client.clone();
//             let cache_path = session.get_config().cache_path.join(format!(
//                 "{}#{}#{}",
//                 package.name,
//                 package.version(),
//                 fs::filenamify(url)
//             ));

//             if !no_cache && cache_path.exists() {
//                 continue;
//             }

//             // strip `#/dl.7z` url renaming
//             let url = url.split_once('#').map(|s| s.0).unwrap_or(url);

//             let mut request = client.get(url);

//             // Add cookie header if present
//             if let Some(cookie) = cookie {
//                 let mut cookies = vec![];
//                 for (key, value) in cookie {
//                     cookies.push(format!("{}={}", key, value));
//                 }
//                 let cookie = cookies.join("; ");
//                 request = request.set("Cookie", &cookie);
//             }

//             let response = request.call()?;

//             if response.status() != 200 {
//                 return Err(Error::Custom(format!(
//                     "failed to fetch {} (status code: {}",
//                     url,
//                     response.status()
//                 )));
//             }

//             let content_length = response
//                 .header("Content-Length")
//                 .map(|s| s.parse::<u64>().unwrap_or_default())
//                 .unwrap_or_default();
//             let accept_ranges = response
//                 .header("Accept-Ranges")
//                 .map(|s| "bytes" == s)
//                 .unwrap_or_default();

//             let cache_file = CacheFile::from(cache_path)?;

//             let ctx = DownloadProgressContext {
//                 name: package.ident(),
//                 total: content_length,
//                 position: 0,
//                 file_count,
//                 index,
//                 state: DownloadProgressState::Prepared,
//             };

//             let (tx, rx) = mpsc::channel::<ChunkedRange>();
//             let mut tasks = vec![];
//             if !accept_ranges {
//                 let pool = ThreadPool::builder().pool_size(2).create()?;

//                 let write_task = pool
//                     .spawn_with_handle(do_write(cache_file, ctx, rx, callback.clone()))
//                     .map_err(|e| Error::Custom(e.to_string()))?;
//                 tasks.push(write_task);

//                 let read_task = pool
//                     .spawn_with_handle(do_read(response, tx.clone()))
//                     .map_err(|e| Error::Custom(e.to_string()))?;
//                 tasks.push(read_task);
//             } else {
//                 let default_connections = 5;
//                 let split_size = 5_000_000 as u64;

//                 let x = content_length;
//                 let y = split_size;

//                 let split_count = (x / y + (x % y != 0) as u64) as usize;
//                 let connections = std::cmp::min(split_count, default_connections);

//                 let mut ranges = vec![];
//                 let mut range_start = 0;
//                 let mut range_end = 0;
//                 for _ in 1..=split_count {
//                     range_end += split_size;
//                     if range_end >= content_length {
//                         range_end = content_length - 1;
//                     }
//                     ranges.push((range_start, range_end));
//                     range_start = range_end + 1;
//                 }

//                 let pool_size = connections + 1;
//                 let pool = ThreadPool::builder().pool_size(pool_size).create()?;

//                 let write_task = pool
//                     .spawn_with_handle(do_write(cache_file, ctx, rx, callback.clone()))
//                     .map_err(|e| Error::Custom(e.to_string()))?;
//                 tasks.push(write_task);

//                 for range in ranges {
//                     let mut request = client.get(url);
//                     request = request.set("Range", &format!("bytes={}-{}", range.0, range.1));
//                     let read_task = pool
//                         .spawn_with_handle(do_read_range(request, range, tx.clone()))
//                         .map_err(|e| Error::Custom(e.to_string()))?;
//                     tasks.push(read_task);
//                 }
//             }
//             drop(tx);

//             let joined = futures::future::join_all(tasks);
//             futures::executor::block_on(joined);
//         }
//     }

//     Ok(())
// }

// async fn do_write<F>(
//     cache_file: CacheFile,
//     mut ctx: DownloadProgressContext,
//     rx: Receiver<ChunkedRange>,
//     callback: Arc<Mutex<F>>,
// ) -> Fallible<()>
// where
//     F: FnMut(DownloadProgressContext),
// {
//     let mut callback = callback.lock().unwrap();

//     let fd = std::fs::OpenOptions::new()
//         .truncate(true)
//         .create(true)
//         .write(true)
//         .open(cache_file.path())?;

//     // emit
//     callback(ctx.clone());

//     while let Ok(chunk) = rx.recv() {
//         let _ = fd.seek_write(&chunk.data[..chunk.length as usize], chunk.offset)?;

//         ctx.position = ctx.position + chunk.length;
//         if ctx.state != DownloadProgressState::Downloading {
//             ctx.state = DownloadProgressState::Downloading;
//         }
//         callback(ctx.clone());
//     }
//     drop(fd);

//     ctx.state = DownloadProgressState::Finished;
//     // emit
//     callback(ctx);
//     Ok(())
// }

// async fn do_read(response: ureq::Response, tx: Sender<ChunkedRange>) -> Fallible<()> {
//     let mut chunk = [0; 4096];
//     let mut offset = 0;
//     let mut reader = response.into_reader();

//     loop {
//         match reader.read(&mut chunk)? {
//             0 => break,
//             len => {
//                 let chunk = ChunkedRange {
//                     offset,
//                     length: len as u64,
//                     data: chunk,
//                 };
//                 offset += len as u64;
//                 tx.send(chunk).unwrap();
//             }
//         }
//     }
//     Ok(drop(tx))
// }

// async fn do_read_range(
//     request: Request,
//     range: (u64, u64),
//     tx: Sender<ChunkedRange>,
// ) -> Fallible<()> {
//     let response = request.call()?;
//     if !(response.status() >= 200 && response.status() <= 299) {
//         return Err(Error::Custom(format!(
//             "failed to fetch (status code: {})",
//             response.status()
//         )));
//     }

//     let mut chunk = [0; 4096];
//     let mut offset = range.0;
//     let mut reader = BufReader::new(response.into_reader());

//     loop {
//         match reader.read(&mut chunk)? {
//             0 => break,
//             length => {
//                 let chunked_range = ChunkedRange {
//                     offset,
//                     length: length as u64,
//                     data: chunk,
//                 };

//                 tx.send(chunked_range)
//                     .map_err(|e| Error::Custom(format!("failed to send chunk: {}", e)))?;

//                 offset += length as u64;
//             }
//         }
//     }
//     Ok(drop(tx))
// }

// pub(crate) fn verify_integrity(session: &Session, packages: &Vec<Package>) -> Fallible<()> {
//     println!("Verifying integrity of packages...");

//     for package in packages {
//         // skip nightly package
//         if package.manifest.is_nightly() {
//             continue;
//         }

//         let urls = package.manifest.url_with_hash();
//         print!("Checking hash of {}... ", package.name);

//         for (url, hash) in urls.into_iter() {
//             let cache_path = session.get_config().cache_path.join(format!(
//                 "{}#{}#{}",
//                 package.name,
//                 package.version(),
//                 fs::filenamify(url)
//             ));

//             let mut hasher = Checksum::new(hash).map_err(|e| Error::Custom(e.to_string()))?;
//             let mut file = std::fs::File::open(&cache_path)
//                 .with_context(|| format!("failed to open cache file: {}", cache_path.display()))?;
//             let mut buffer = [0; 4096];
//             loop {
//                 let len = file.read(&mut buffer).with_context(|| {
//                     format!("failed to read cache file: {}", cache_path.display())
//                 })?;
//                 match len {
//                     0 => break,
//                     len => hasher.consume(&buffer[..len]),
//                 }
//             }
//             let checksum = hasher.result();
//             if hash != &checksum {
//                 println!("Err");
//                 return Err(Error::Custom(format!(
//                     "checksum mismatch: {}\n Expected: {}\n Actual: {}",
//                     cache_path.display(),
//                     hash,
//                     checksum
//                 )));
//             }
//         }
//         println!("Ok");
//     }

//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Manifest;
    use crate::Session;

    #[test]
    fn downloads_files_with_unknown_remote_size() {
        // Regression test: servers (or schemes like file://) that report no
        // Content-Length must not mark missing files as valid cache, which
        // used to skip the download and fail the integrity check.
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-probe-dl");
        std::fs::create_dir_all(dir.join("cache")).unwrap();
        std::fs::write(dir.join("payload.bin"), b"0123456789").unwrap();
        std::env::set_var("SCOOP_CACHE", dir.join("cache"));

        let url = format!(
            "file:///{}/payload.bin",
            dir.to_string_lossy().replace('\\', "/")
        );
        let json = format!(
            r#"{{"version": "1.0", "homepage": "https://example.com",
                "license": "MIT", "url": "{url}",
                "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}"#
        );
        let manifest = Manifest::parse_bytes(json.as_bytes(), &dir.join("probe.json")).unwrap();
        let session = Session::new();
        assert_eq!(session.config().cache_path(), dir.join("cache").as_path());
        let pkg = Package::from("probe", "main", manifest);
        let pkgs = [&pkg];
        let mut set = PackageSet::new(&session, &pkgs, true).unwrap();
        let size = set.calculate_download_size().unwrap();
        assert_eq!(size.total, 0);
        set.download().unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.join("cache"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1, "cache should contain the download");
        assert_eq!(std::fs::read(&entries[0]).unwrap(), b"0123456789");

        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Build a session whose `PATH` starts with a fake `aria2c.exe` that
    /// always fails (a copy of powershell.exe choking on aria2 arguments),
    /// with aria2 enabled. Returns the session plus the original `PATH`.
    #[cfg(windows)]
    fn failing_aria2_session(dir: &std::path::Path) -> (Session, std::ffi::OsString) {
        let bindir = dir.join("fakebin");
        std::fs::create_dir_all(&bindir).unwrap();
        let system_ps = std::path::PathBuf::from(
            std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string()),
        )
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        std::fs::copy(&system_ps, bindir.join("aria2c.exe"))
            .expect("powershell.exe should be copyable");

        let original_path = std::env::var_os("PATH").unwrap_or_default();
        let mut with_fake = bindir.to_string_lossy().into_owned();
        with_fake.push(';');
        with_fake.push_str(&original_path.to_string_lossy());
        std::env::set_var("PATH", &with_fake);

        // Isolated temp config so `set()` never touches the real config.
        std::env::set_var("SCOOP", dir.join("root"));
        std::env::set_var("SCOOP_GLOBAL", dir.join("global"));
        std::env::set_var("SCOOP_CACHE", dir.join("cache"));
        std::fs::write(dir.join("config.json"), "{}").unwrap();
        let session = Session::new_with(dir.join("config.json")).unwrap();
        assert!(session.config().aria2_fallback_enabled());
        session
            .config_mut()
            .unwrap()
            .set("aria2-enabled", "true")
            .unwrap();
        (session, original_path)
    }

    #[cfg(windows)]
    fn aria2_probe_package(dir: &std::path::Path) -> Package {
        std::fs::create_dir_all(dir.join("cache")).unwrap();
        std::fs::write(dir.join("payload.bin"), b"0123456789").unwrap();
        let url = format!(
            "file:///{}/payload.bin",
            dir.to_string_lossy().replace('\\', "/")
        );
        let json = format!(
            r#"{{"version": "1.0", "homepage": "https://example.com",
                "license": "MIT", "url": "{url}"}}"#
        );
        let manifest = Manifest::parse_bytes(json.as_bytes(), &dir.join("probe.json")).unwrap();
        Package::from("probe", "main", manifest)
    }

    /// A failing aria2c falls back to curl (upstream
    /// `ARIA2-FALLBACK-ENABLED`, default on) instead of aborting.
    #[test]
    #[cfg(windows)]
    fn aria2_failure_falls_back_to_curl() {
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-probe-aria2-fallback");
        let _ = std::fs::remove_dir_all(&dir);
        let (session, original_path) = failing_aria2_session(&dir);
        let pkg = aria2_probe_package(&dir);
        let pkgs = [&pkg];

        let mut set = PackageSet::new(&session, &pkgs, true).unwrap();
        set.download().expect("curl fallback should succeed");

        let entries: Vec<_> = std::fs::read_dir(dir.join("cache"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1, "cache should contain the download");
        assert_eq!(std::fs::read(&entries[0]).unwrap(), b"0123456789");

        std::env::set_var("PATH", &original_path);
        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// With the fallback disabled, an aria2c failure surfaces the exit
    /// code instead of retrying.
    #[test]
    #[cfg(windows)]
    fn aria2_failure_aborts_when_fallback_disabled() {
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-probe-aria2-nofallback");
        let _ = std::fs::remove_dir_all(&dir);
        let (session, original_path) = failing_aria2_session(&dir);
        session
            .config_mut()
            .unwrap()
            .set("aria2-fallback-enabled", "false")
            .unwrap();
        assert!(!session.config().aria2_fallback_enabled());
        let pkg = aria2_probe_package(&dir);
        let pkgs = [&pkg];

        let mut set = PackageSet::new(&session, &pkgs, true).unwrap();
        let err = set.download().unwrap_err();
        assert!(
            matches!(err, crate::Error::Aria2 { .. }),
            "expected structured aria2 error, got: {err}"
        );

        std::env::set_var("PATH", &original_path);
        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&dir).ok();
    }
}
