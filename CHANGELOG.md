# Changelog

## [0.1.1-beta.10](https://github.com/vincentmathis/bagger/compare/v0.1.0-beta.10...v0.1.1-beta.10) (2026-10-07)


### Bug Fixes

* install.ps1 asset resolution, version check, arg forwarding ([27ab12d](https://github.com/vincentmathis/bagger/commit/27ab12da8568ead3f7d5e753615a422edb058dc8))

## 0.1.0-beta.10 (2026-10-07)


### ⚠ BREAKING CHANGES

* **libscoop:** switch to use `tracing` for logging
* **libscoop:** `manifest.hash()` return type changed from `str` to `HashString`
* **libscoop|config:** `Package::manifest_path` is replaced by `manifest().path()`.
* **libscoop:** `SyncOption::NoDownloadSize` becomes `SyncOption::Offline`
* **libscoop:** Some `Event` variants related to bucekt update progress have been updated to fit the latest codebase.
* **libscoop:** `Session::new()` is now infallible.
* **libscoop:** APIs of operations and Session changed.
* **libscoop:** exposed modules of libscoop changed.

### Features

* add 'bagger alias', 'bagger export', 'bagger import' commands ([9f4ae94](https://github.com/vincentmathis/bagger/commit/9f4ae94d6ff3f7348553c4ba1a113d981edb4dc7))
* add 'bagger checkup' command ([232b63f](https://github.com/vincentmathis/bagger/commit/232b63f48048a35567df50d995fe6f29e3483797))
* add 'bagger checkver' command ([a7acf9b](https://github.com/vincentmathis/bagger/commit/a7acf9b1e28f4cda0ebb37491612832b548261f7))
* add 'bagger create', 'bagger depends', 'bagger download' commands ([0c19c45](https://github.com/vincentmathis/bagger/commit/0c19c45709c9b8f20339c3fc591804028579c314))
* add 'bagger prefix' command ([881f118](https://github.com/vincentmathis/bagger/commit/881f118043547d0b859621b099fa424a03514cf4))
* add 'bagger reset' command ([d458072](https://github.com/vincentmathis/bagger/commit/d458072b396f36432615a6a17b869352ab50ac0e))
* add 'bagger shim' command ([32c7967](https://github.com/vincentmathis/bagger/commit/32c79678ac8d7c85fbe8919fbced73e91e83904a))
* add 'bagger virustotal' command (simulation) ([ba0cf67](https://github.com/vincentmathis/bagger/commit/ba0cf6708bcbc4f6ea1c2834e98ca20729648fce))
* add 'bagger which' and 'bagger status' commands ([604a164](https://github.com/vincentmathis/bagger/commit/604a1647e15420e4308344c5cce371b11c568c0f))
* add 'bagger which' command ([15e9332](https://github.com/vincentmathis/bagger/commit/15e9332c3573199b3324ef8e1b7d0db4e0201a20))
* add hash crate ([aa021fb](https://github.com/vincentmathis/bagger/commit/aa021fb7fa6eaa3167f803608982307ebbafe9f7))
* add opt-in SQLite manifest cache for bucket queries ([c760a2c](https://github.com/vincentmathis/bagger/commit/c760a2c50c3bee49dd31ae599063cd1a336ddb07))
* alias persistence, shim rm alias, create bucket layout ([b44da4c](https://github.com/vincentmathis/bagger/commit/b44da4c2205b900bda53f27b652b91027be8440d))
* **api:** Introduce SPDX spec for manifest.license ([ec5e1f5](https://github.com/vincentmathis/bagger/commit/ec5e1f5c6286100724f346ab55ab7fc11d02d5fe))
* autoupdate extract/json/xpath hash modes per upstream ([6dd62dc](https://github.com/vincentmathis/bagger/commit/6dd62dc37d9350925d9e266ce9860cfaebad86d6))
* autoupdate site hash modes (fosshub/sourceforge/github/rdf) ([bde66ce](https://github.com/vincentmathis/bagger/commit/bde66cea32d945546a656c57336db136cc5ad1c1))
* bucket-priority candidate auto-selection ([24a036f](https://github.com/vincentmathis/bagger/commit/24a036fcd3c5e78f1f9597fbe58a4e2db11ceea6))
* **cache:** implement cache-rm ([869f095](https://github.com/vincentmathis/bagger/commit/869f0956a0ccb6a8dc06d40d95bde9f79b09e504))
* checkver reverse/replace/captures with Scoop-faithful chaining ([17510e6](https://github.com/vincentmathis/bagger/commit/17510e67f3c2592edae40c66d5b4888aeedcb921))
* checkver script support, drop bogus cache-clean row ([7817054](https://github.com/vincentmathis/bagger/commit/7817054b1b15531d1bcf2d3559e2693e1543c52e))
* **cmd:** Implement cleanup, refactor cache and list ([3ca3f26](https://github.com/vincentmathis/bagger/commit/3ca3f2610ec5bf0164bfde1d4f91484423cc78c4))
* **cmd:** Implement scoop list subcommand ([0b2fdec](https://github.com/vincentmathis/bagger/commit/0b2fdec835835b68b500a19d450f39e82c08a4b6))
* **cmd:** prototype of scoop home ([29c2663](https://github.com/vincentmathis/bagger/commit/29c2663768e7bed616e104c6a5339b55bcdf7536))
* **cmd:** prototype of scoop info ([a207465](https://github.com/vincentmathis/bagger/commit/a207465b73a704ef31014ccd408c323c45cbbdb5))
* **cmd:** prototype of scoop search (local) ([2c8e563](https://github.com/vincentmathis/bagger/commit/2c8e563748539b63e6c95b9c09dbe9b1b1995199))
* complete autofetch hash rewriting for download mode ([23ec43d](https://github.com/vincentmathis/bagger/commit/23ec43d063ff681226e61fc2c61e5297317afab0))
* **core:** add DepGraph implementation ([ca5f49f](https://github.com/vincentmathis/bagger/commit/ca5f49fcd23437a5d257fd83fd23cf1c512cdb27))
* **core:** Implement update subcommand ([ad04e76](https://github.com/vincentmathis/bagger/commit/ad04e76762de55954d070be3a3a352b29a78981e))
* evaluate checkver xpath via roxmltree subset evaluator ([e1090a8](https://github.com/vincentmathis/bagger/commit/e1090a8910b7fcd5ef0deffbd03d5620f96dc34c))
* export/import installed architecture, fix query_installed regression ([94c1f94](https://github.com/vincentmathis/bagger/commit/94c1f940f7ed14ddf95936e680f3d4ef857eb218))
* **hash-md5:** add reset api ([3db7116](https://github.com/vincentmathis/bagger/commit/3db7116412729ff2ca84de93ecd1a1850e17100e))
* **hash:** add checksum helper functions ([df24980](https://github.com/vincentmathis/bagger/commit/df24980c664699b24a2efc7b609c2ba324521333))
* **hash:** add sha1 implementation ([8bee89a](https://github.com/vincentmathis/bagger/commit/8bee89ae49f30cfbdb42c76c52331c7fd5ba8b82))
* **hash:** add sha256 implementation ([37f9f62](https://github.com/vincentmathis/bagger/commit/37f9f622e79a5ec4d3bf122ccafd191d46041c2b))
* **hash:** add sha512 implementation ([7bbecf1](https://github.com/vincentmathis/bagger/commit/7bbecf1310ee342e4d4376e413f852b16f6aadd2))
* **hash:** provided a top-level checksum api ([99fed09](https://github.com/vincentmathis/bagger/commit/99fed093d48d5cf91f3db0f46f02c4d152d17043))
* **hok|cat,home:** support candidate selection ([28b56c5](https://github.com/vincentmathis/bagger/commit/28b56c5ade13e1edceb04fa7c0fc7554dcc0c6a9))
* **hok|cat:** show manifest path ([7e06467](https://github.com/vincentmathis/bagger/commit/7e064672ebd6aa2009f1db49ea6a0f8704139be3))
* **hok|config:** config-list shows the path ([679c177](https://github.com/vincentmathis/bagger/commit/679c1771c036982941bce62e6db55e9098b4e739))
* **hok:** add `hok completions` command to generate shell completion ([da1b6d8](https://github.com/vincentmathis/bagger/commit/da1b6d8f409d8c7894872dab84e28cb8d1814fab))
* **hok:** add global `--verbose` flag ([5fd0505](https://github.com/vincentmathis/bagger/commit/5fd050584e80687452a6dde798824cd312e1b74a))
* **hok:** add uninstall cmd placeholder ([c13e8be](https://github.com/vincentmathis/bagger/commit/c13e8be627ab0bfb91aedfebc10ee89dc2ee8675))
* **hok:** added s shortcut for search command ([50c0bfc](https://github.com/vincentmathis/bagger/commit/50c0bfcd6dd928dc105a4ec7afefb1d4e0aa97c7))
* **hok:** reflect basic support of uninstalling packages ([183cfd8](https://github.com/vincentmathis/bagger/commit/183cfd8b54e8e96ce2e575240f3b7edb3183f005))
* **hok:** show bucket manifest count ([d71e193](https://github.com/vincentmathis/bagger/commit/d71e193be2cc20598e53b08947635f67a1409399))
* **hok:** support list held packages ([a2acb22](https://github.com/vincentmathis/bagger/commit/a2acb2210bf0586f6d839d61773b1dac7d2f96f1))
* **hok:** support resolving and downloading packages ([bdc08dd](https://github.com/vincentmathis/bagger/commit/bdc08dd63898f7af22fa538f20b3fb068e87c26f))
* honor checkver.useragent, quiet absent shim removals ([7b326d5](https://github.com/vincentmathis/bagger/commit/7b326d582fe62fe22fe292ece860bdd5200883aa))
* implement autofetch, runtime deps, running-process guard, real VirusTotal lookup ([da6c760](https://github.com/vincentmathis/bagger/commit/da6c760fd977664f253751d9ddbd20f7441d256d))
* Implement basic file downloads ([c5d303b](https://github.com/vincentmathis/bagger/commit/c5d303bff23993ca4bc53946c074058a542a0420))
* implement hold and unhold ([682c63c](https://github.com/vincentmathis/bagger/commit/682c63c78390ee4300a6c9ad42934b79be7b5866))
* implement install commit step ([85f4b09](https://github.com/vincentmathis/bagger/commit/85f4b099918e8f74a61bc3e94cb54fe152e2907a))
* implement status ([bb650d6](https://github.com/vincentmathis/bagger/commit/bb650d64c711f74ff1f73c3026b86c90daafe14b))
* install notes/suggestions, checkver jsonpath + arch fallback ([5b7f1a7](https://github.com/vincentmathis/bagger/commit/5b7f1a783b0a12a0667db3736e57feaa08241895))
* integrate aria2c downloader with curl fallback ([fe79cc0](https://github.com/vincentmathis/bagger/commit/fe79cc0253a4f03a42204f57b853d068603db48d))
* **libscoop|config:** support `SCOOP_CACHE` and `SCOOP_GLOBAL` envs ([cf2a2a5](https://github.com/vincentmathis/bagger/commit/cf2a2a5503c93e5d57b5ac72aec490e2d53b2a7d))
* **libscoop|config:** support `use_isolated_path` config ([1bb5ee7](https://github.com/vincentmathis/bagger/commit/1bb5ee773867c490af8e21885acc87e84a33f40c))
* **libscoop|download:** support injecting cookie defined in manifest ([aec7fdc](https://github.com/vincentmathis/bagger/commit/aec7fdc851aee1673170182f7d382a069d514649))
* **libscoop|download:** write to temp file in downloading ([d79e598](https://github.com/vincentmathis/bagger/commit/d79e5989aa01b1d49cc02003692e2f4b46991ca0))
* **libscoop|event:** added integrity check event and error type ([888afbb](https://github.com/vincentmathis/bagger/commit/888afbba203b80dfd4accf57fbc99dc1b348d3e3))
* **libscoop|manifest:** impl Display for License ([e91ff0e](https://github.com/vincentmathis/bagger/commit/e91ff0ec48a295a91d771e7256e542e9cab74846))
* **libscoop|manifest:** support aarch64 specific fields ([639d092](https://github.com/vincentmathis/bagger/commit/639d092e22dc32decc98950532614da75489dbe6))
* **libscoop|resolve:** added `resolve_cascade` ([0aa0c52](https://github.com/vincentmathis/bagger/commit/0aa0c52802ea2238a31352e9ae0b19c730b7510e))
* **libscoop|resolve:** added fn `select_candidate` ([0e296ea](https://github.com/vincentmathis/bagger/commit/0e296ea5b0cb2ab884c74ccea42df86ca05840e0))
* **libscoop|resolve:** allow to select installed candidate ([8fb0ec3](https://github.com/vincentmathis/bagger/commit/8fb0ec39509128498be1bcbeb3fcddb5edb16838))
* **libscoop|sync:** added SyncOption::EscapeHold for package remove ([ca8fad7](https://github.com/vincentmathis/bagger/commit/ca8fad7ffbd1dd1cb0a6d1e03f924e63c5db3364))
* **libscoop|sync:** basic support of uninstalling packages ([b1f0f6b](https://github.com/vincentmathis/bagger/commit/b1f0f6bd3c7ee61b846d60a70889c4033730b10a))
* **libscoop:** add package resolving and event bus ([434eebe](https://github.com/vincentmathis/bagger/commit/434eebe3d464edb48a1d034d4e746810ba41d274))
* **libscoop:** added coordination between `AssumeYes` and `NoDownloadSize` ([5e9d578](https://github.com/vincentmathis/bagger/commit/5e9d5784f62fd0eb64009aa23d6d76847c164f46))
* **libscoop:** added package integrity check logic ([57869f7](https://github.com/vincentmathis/bagger/commit/57869f763e5a1a9c3668b3028d46787e5ce0e04d))
* **libscoop:** added support for package resolution and download ([4ff0d95](https://github.com/vincentmathis/bagger/commit/4ff0d9573794c003c440477656e808bd527377a2))
* **libscoop:** Adpot new cache filename format ([15172a9](https://github.com/vincentmathis/bagger/commit/15172a9f7ac35963d1f274e51a4a72de478546c1))
* **libscoop:** impl Default for Session ([d91177a](https://github.com/vincentmathis/bagger/commit/d91177a269698b8fbd7b530f0100da82d4ce8879))
* **libscoop:** remove env paths under isolated_path mode correctly ([2f58173](https://github.com/vincentmathis/bagger/commit/2f5817387006a9334f5a74bc7c17d7063e529108))
* **libscoop:** replace ureq with libcurl ([7d3df7c](https://github.com/vincentmathis/bagger/commit/7d3df7c3e954187318d46958f07d6e4b4ce9fe31))
* **libscoop:** scoop-hash features passthrough ([cb027ce](https://github.com/vincentmathis/bagger/commit/cb027cedd98de15aa17602234b824b240c2fcc2c))
* **libscoop:** support `use_sqlite_cache` config ([35c9577](https://github.com/vincentmathis/bagger/commit/35c9577be0bf497e23c5857e350b7b5717b35645))
* **libscoop:** support loading config from all possible location ([2bcc649](https://github.com/vincentmathis/bagger/commit/2bcc649808e8238bef5795c73eab41c182cac61b))
* make import actually install exported apps ([7b0a9d5](https://github.com/vincentmathis/bagger/commit/7b0a9d5e6d5558aaf543cade3531038759f222c3))
* move to v0.1.0-alpha.2 ([24e354a](https://github.com/vincentmathis/bagger/commit/24e354a7514d74878c550e25457d323e6251ee4b))
* move to v0.1.0-alpha.3 ([1ecd0ed](https://github.com/vincentmathis/bagger/commit/1ecd0edf100ea4a3676494b40b5c72c787ad5501))
* move to v0.1.0-beta.1 ([e1a2376](https://github.com/vincentmathis/bagger/commit/e1a2376e58eb91889d7b102aaa6c415cf7b49ef1))
* runtime architecture override via --arch / SCOOP_ARCH ([f348d48](https://github.com/vincentmathis/bagger/commit/f348d4898ef1eeee98c873f7c67e0c268741e000))
* schema-compat hardening from upstream schema audit ([6c5a72c](https://github.com/vincentmathis/bagger/commit/6c5a72cc9b626201628ddfa5fed172819f6c7d1f))
* scoop hook-scope prelude (Expand-* + vars); upstream install ordering; installer.file; native msi extraction ([ebc968d](https://github.com/vincentmathis/bagger/commit/ebc968dfba6410d6ed4cfaafa8bc0909c4e8642c))
* **scoop-cache:** implement scoop cache show ([ae018b8](https://github.com/vincentmathis/bagger/commit/ae018b86a3abfe23d4f6f9c17edc9047947af8e4))
* **scoop-cache:** implement scoop cache show ([c584c90](https://github.com/vincentmathis/bagger/commit/c584c90ff3e90e8744841ea64e3f732a29571b55))
* **scoop-config:** implement scoop config ([9bdc9fa](https://github.com/vincentmathis/bagger/commit/9bdc9fa8a46897dea3aef636bd92d51a27b7616f))
* **scoop-hash:** support switching hashing backend ([d38658e](https://github.com/vincentmathis/bagger/commit/d38658ef8785df92189b29df7094dadfc609e14c))
* **scoop-hash:** use builder pattern ([87ca347](https://github.com/vincentmathis/bagger/commit/87ca3475bd4d5cb947c4ee2702807f944d92c729))
* **search:** Add fuzzy search option ([53c8998](https://github.com/vincentmathis/bagger/commit/53c8998ed98b4a150e19ffb4a10ce7a7e8ab160e))
* **search:** Implement binary search ([26ff6f2](https://github.com/vincentmathis/bagger/commit/26ff6f248fc323f5be1e168e10d19fff613e07a9))
* support global installs via session root override ([0aea883](https://github.com/vincentmathis/bagger/commit/0aea88336e439c0b22c07b3b97370624a72a7fef))
* support isolated installs from manifest URLs and files ([4231ab9](https://github.com/vincentmathis/bagger/commit/4231ab90a05e203d6a861836a0f57473fe9ed5f0))
* update ([cf505f0](https://github.com/vincentmathis/bagger/commit/cf505f0e51ac4c5777e260651d0ee0cd5e805abb))
* v0.1.0-alpha.1 ([f304bb2](https://github.com/vincentmathis/bagger/commit/f304bb262dc1f850ae3932bb810ab91ee272fd2b))
* verify archive/script/persist e2e, fix checkup broken-shim scan ([79ba5eb](https://github.com/vincentmathis/bagger/commit/79ba5ebf8655dc85806e018505d638721526d9a6))


### Bug Fixes

* **bucket-rm:** use remove_dir_all crate ([a5c9a0b](https://github.com/vincentmathis/bagger/commit/a5c9a0bb309a54bae2e80552d4a5c9c0b5a4ef16))
* cargo clippy fix ([71a7596](https://github.com/vincentmathis/bagger/commit/71a759605a5ea2b68093cf19f1f3fc8bfe3c15b8))
* **ci:** remove unneeded condition ([718084f](https://github.com/vincentmathis/bagger/commit/718084f80c615513c69a838205e58edd2a553d44))
* cleanup continues past locked versions; hardened persist unlink; running-process hint ([7390360](https://github.com/vincentmathis/bagger/commit/7390360829170ce69f0777731fd0fcca5aab6531))
* clippy warnings in checkup, import, create, depends ([4121831](https://github.com/vincentmathis/bagger/commit/4121831f508dd38c8c2ace9864b3a263965ef95c))
* **core:** fix cache regex ([98f2a44](https://github.com/vincentmathis/bagger/commit/98f2a44d872c876c6925e6a5ffadbc4864ddfb71))
* **core:** fix manifest download urls extraction ([7fef94c](https://github.com/vincentmathis/bagger/commit/7fef94cb1235ce446d10bf0ab09bc853fc1ccd0e))
* download -v collision, junction-aware reset ([1bdc906](https://github.com/vincentmathis/bagger/commit/1bdc90630b962726b0a0b6807d6d80a2edaca527))
* download unknown-size skip, shim add/remove symmetry ([f65b2c4](https://github.com/vincentmathis/bagger/commit/f65b2c4fad7d7899e76f2f91094245c43bf431f4))
* drop unimplemented download --version flag ([0687429](https://github.com/vincentmathis/bagger/commit/068742917ea76d8d04452b64d2d7d2749a7b6709))
* Fix apps_in_local_bucket ([b3170c7](https://github.com/vincentmathis/bagger/commit/b3170c72263dabacb027e8b66b1be3ce7113bfb7))
* fix cache rm handler ([22f51e2](https://github.com/vincentmathis/bagger/commit/22f51e2cb82a80d1123895452ed2cecbf0d09b4a))
* Fix not truncating previous data when saving new configs ([ea4bf0c](https://github.com/vincentmathis/bagger/commit/ea4bf0c1fa28d7ede20d35cedc5423c515a4a029))
* harden filesystem-name handling, drop dead downloader ([dfec4f3](https://github.com/vincentmathis/bagger/commit/dfec4f3d590bf519e331b0e5f329d6e55d33ae5a))
* **hok|cat,home:** sort candidates ([c90f3f9](https://github.com/vincentmathis/bagger/commit/c90f3f94367dae75cabd2dd0a562f38c924f6dbd))
* **hok|list:** only print upgradable when the flag is used ([558d9d3](https://github.com/vincentmathis/bagger/commit/558d9d39603d85657986da43f8c98372ac938e30))
* **hok:** accumulate downloaded bytes properly ([d6fabc8](https://github.com/vincentmathis/bagger/commit/d6fabc89aa0bfa1428328bddc248c11fe2e9d8e9))
* **hok:** added long format arg of listing known buckets ([658ef7d](https://github.com/vincentmathis/bagger/commit/658ef7d9e799301bbd5807a195dd2f263933d5c1))
* **hok:** Check if `cat_style` is empty ([4ed662e](https://github.com/vincentmathis/bagger/commit/4ed662ed8bc5d35e3570a8daf1bb6fd92c429f40)), closes [#10](https://github.com/vincentmathis/bagger/issues/10)
* **hok:** fix 50c0bfc ([387fd66](https://github.com/vincentmathis/bagger/commit/387fd66637d7e53d167a18be8a0fc9daf121475e))
* **hok:** print ending newline for error report ([d1f5682](https://github.com/vincentmathis/bagger/commit/d1f56822a1db93cf566265b3ec44082794896422))
* **hok:** Remove `type.exe` dep in `hok cat` ([47a42c5](https://github.com/vincentmathis/bagger/commit/47a42c57c17276c7024baaf13479b88e4eac81a1))
* **hok:** trim yes_no prompt input ([0a01f1e](https://github.com/vincentmathis/bagger/commit/0a01f1e1ee65e50f9cc5e081d8daa734ea7770e4))
* **libscoop|config:** correct `no_junction` field ([4bae700](https://github.com/vincentmathis/bagger/commit/4bae700efa06b4d07506370e2eaace04ef747d3d))
* **libscoop|config:** default config path should be always returned ([d3040ad](https://github.com/vincentmathis/bagger/commit/d3040adf732839bb0070f6585be5428ad0d25e73))
* **libscoop|config:** support named `use_isolated_path` ([5e9a181](https://github.com/vincentmathis/bagger/commit/5e9a18135ea309248e657c44bcf43a235833a7cf))
* **libscoop|fs:** `write_json` should create file instead of dir ([54482a7](https://github.com/vincentmathis/bagger/commit/54482a7c8c1733e8d0c01bac5e85fc5da7f4fd3e))
* **libscoop|fs:** improve symlink removal logic ([398ef27](https://github.com/vincentmathis/bagger/commit/398ef27fc280ded401e0e5fb5a9123d5a165b2af))
* **libscoop|query:** don't create empty apps dir ([7287bd5](https://github.com/vincentmathis/bagger/commit/7287bd5672f6eb88ebb52acb928bfbcc6e87877a))
* **libscoop|resolve:** correct pinned dependency cascade resolving ([660d3e2](https://github.com/vincentmathis/bagger/commit/660d3e2da5bbe5218c45c8706282bfdbc2bfe760))
* **libscoop:** added portability on non-windows ([3d1ffee](https://github.com/vincentmathis/bagger/commit/3d1ffeeb39074a8c31cbb97891c082fd2a31a7fc))
* **libscoop:** avoid forcing doc target as it will fail to build ([7674f8a](https://github.com/vincentmathis/bagger/commit/7674f8aef2f1cdf952c96aed6f16dbb08f65f335))
* **libscoop:** case insensitive match on package querying ([5efde66](https://github.com/vincentmathis/bagger/commit/5efde6661999bd6c7f6bff8d1d9e30d78db5564e))
* **libscoop:** dag should check self cyclic ([a5bbb0b](https://github.com/vincentmathis/bagger/commit/a5bbb0bb5f57ef6d8d326e6eca9bb828f6ff6ec9))
* **libscoop:** emit BucketUpdateDone event despite zero bucket ([dc4bdca](https://github.com/vincentmathis/bagger/commit/dc4bdca39bec812073ca68f332d568e391736ef8))
* **libscoop:** ensure cache dir exist before downloading ([485255e](https://github.com/vincentmathis/bagger/commit/485255e926df58efd6e03881d430c8496c9a4adb))
* **libscoop:** ensure ops working dir exist ([0520ae8](https://github.com/vincentmathis/bagger/commit/0520ae8fc6e7e560e343a4dffa4c7b514adf92c3))
* **libscoop:** fix doctest ([c0237a2](https://github.com/vincentmathis/bagger/commit/c0237a2e73d976c4f959bb0928da4cbd0ff3376e))
* **libscoop:** handle wildcard query in upgrade operation ([639e8c6](https://github.com/vincentmathis/bagger/commit/639e8c6680f53c34fbd989fdb60a0ea5e9b92c14))
* **libscoop:** hash checking should be case insensitive (fix [#18](https://github.com/vincentmathis/bagger/issues/18)) ([b3afbef](https://github.com/vincentmathis/bagger/commit/b3afbef0fd438844af786fa2484fb314d7da0227))
* **libscoop:** package resolving is infallible when OnlyUpgrade is used ([cabd52b](https://github.com/vincentmathis/bagger/commit/cabd52bdb1659bb835ad60d9074b8bbdaf345ad0))
* **libscoop:** set install state for package's upgradable reference ([63a54f7](https://github.com/vincentmathis/bagger/commit/63a54f7a36ac1cdcb612437d05c752a97ed9a9e3))
* **libscoop:** update crate categories metadata ([8d6271d](https://github.com/vincentmathis/bagger/commit/8d6271d208c40faf2de32787fe9c5ccf32e303f6))
* **libscoop:** update doc comments ([bcd29b4](https://github.com/vincentmathis/bagger/commit/bcd29b4b172f7adf5511de457f13ce74ac676370))
* **libscoop:** updated Config struct ([17e474a](https://github.com/vincentmathis/bagger/commit/17e474a48a03f3c5281af203802a26f79e284e95))
* **libscoop:** use upgradable package reference when available ([9dfd93f](https://github.com/vincentmathis/bagger/commit/9dfd93fcf58980a113bec1eb781f414c30489de9))
* report missing cache files clearly in integrity check ([a893e3d](https://github.com/vincentmathis/bagger/commit/a893e3d7fd1b3c8394bc3a1d0b4c1332891cb277))
* **scoop-hash:** remove docsrs target ([b6ddd19](https://github.com/vincentmathis/bagger/commit/b6ddd19f7c1f70754c70c3b1c6ca87c43e0e0754))
* skip (don't abort) bulk transactions on running apps; upstream persist upgrade; shim exe poisoning + arg forwarding; 7z resolution ([21ff95c](https://github.com/vincentmathis/bagger/commit/21ff95cbe639e6160b3cf7f049ec67a24447595f))
* stage downloads under real URL basenames; conservative cleanup; missing shim-target warnings; arch extract_to ([d400c37](https://github.com/vincentmathis/bagger/commit/d400c373af65b5d99db33ca2ff73f6b80c9d48c4))
* typo ([4ddc72f](https://github.com/vincentmathis/bagger/commit/4ddc72f944d1fa235fd9644e9ec7896cf917ccc3))
* use init method to create config instance ([9aa08ba](https://github.com/vincentmathis/bagger/commit/9aa08ba9caedefb62b86c4c70593463aacaeefae))


### Performance Improvements

* don't update install info if it's not held ([d8b71b1](https://github.com/vincentmathis/bagger/commit/d8b71b117d97380085b14c145a59495e0ccae5f3))
* **hash-md5:** use inline fn for performance ([7f35660](https://github.com/vincentmathis/bagger/commit/7f356602a93091da5b0017de3a0cb00b3a0e1bb4))
* **libscoop|manifest:** defer hash validation ([d1ff3f6](https://github.com/vincentmathis/bagger/commit/d1ff3f61a46b930771b0d4809fcf77ada2ac04c3))
* **libscoop:** 5x speedup on package querying ([90a8815](https://github.com/vincentmathis/bagger/commit/90a881550df4c3196cd185ab34e4621f854a41b7))
* search enhancement ([13075cc](https://github.com/vincentmathis/bagger/commit/13075cc11a267d98296541fdaf3582b2f9f50eca))


### Miscellaneous Chores

* **libscoop:** tweak exposed modules ([f31cb64](https://github.com/vincentmathis/bagger/commit/f31cb64d3794edf01b55757bb3ecdc19d4878932))


### Code Refactoring

* **libscoop:** switch to use `tracing` for logging ([d835c7f](https://github.com/vincentmathis/bagger/commit/d835c7fb96db2e99ff1b726bb3e1e5f68b31c2f7))

## [0.1.0-beta.10](https://github.com/vincentmathis/bagger/compare/v0.1.0-beta.9...v0.1.0-beta.10) (unreleased)

Scoop hook-script compatibility: manifest scripts now run in an upstream-faithful scope, and installs survive running apps, persist data, and poisoned shims.

### Features

- **hook scope:** every manifest script gets upstream variables (`$dir`, `$version`, `$architecture`, `$app`, `$bucket`, `$bucketsdir`, `$fname`, `$global`, `$original_dir`, `$persist_dir`, `$cmd`) plus ports of `Expand-7zipArchive`/`Msi`/`Inno`/`Dark`/`Zip`, `Get-HelperPath`, `Invoke-ExternalCommand`, `movedir`, `Add/Remove-Path`, env vars, and msg helpers
- **running processes:** bulk install/upgrade/uninstall skips apps with running processes (with a message) instead of aborting; explicit singles still fail; re-checked at commit time
- **installer.file:** file-based `installer`/`uninstaller` entries execute with `$dir`/`$global`/`$version` substitution (`.ps1` via hook scope, else direct), removed unless `keep`
- **shim refresh:** new `bagger shim refresh` repairs executable shims for all installed apps; `BAGGER_FLAVOR=heavy` swaps progress lines (opt-in)

### Bug Fixes

- **extract:** `.msi` goes through lessmsi/msiexec (7z mangles MSI names; MSIs were never extracted); `extract_dir` promotes the subdir up (was inverted and crashed); downloads stage under real URL basenames (`url_filename` semantics)
- **persist:** existing store wins on upgrade (fresh content stashed as `.original`) instead of renaming over it (os error 5); junction unlink goes through the hardened helper; `cleanup` never deletes metadata-less version dirs and continues past locked ones with an in-use hint
- **shim:** never write batch content into `{name}.exe` (os error 216, 77 apps affected); all flavors forward caller args (`%*`/`"$@"`/`@args`); missing targets warn loudly
- **ci:** fix `outputs` block indentation (was an invalid workflow); bump `release-as` to `0.1.0-beta.10`

---

## [0.1.0-beta.9](https://github.com/vincentmathis/bagger/compare/v0.1.0-beta.8...v0.1.0-beta.9) (2026-10-06)

### Bug Fixes

- **ci:** fix release workflow repository gate (`chawyehsu/hok` → `vincentmathis/bagger`) and bump `release-as` to `0.1.0-beta.9`
- **install:** correct README install instructions (point at `raw.githubusercontent.com` instead of fictional `bagger.sh`)
- **ci:** publish artifacts as `bagger-windows-*` instead of `hok-windows-*`

---

## [0.1.0-beta.8](https://github.com/vincentmathis/bagger/compare/v0.1.0-beta.7...v0.1.0-beta.8) (2026-10-06)

Full Scoop command parity (31 commands): every Scoop command now has a `bagger` equivalent.

### ⚠ BREAKING CHANGES

- **crates:** `hok` renamed to `bagger`; `libscoop` renamed to `scoop-rs`; `scoop-hash` renamed to `bagger-hash`. The original `scoop-hash` crate on crates.io was owned by a third party, so the crate was renamed.
- All crate names, crate IDs, and crate-internal module paths changed. Update imports accordingly.

### Features

- **all:** rebrand to `bagger` and `scoop-rs`; new `bagger-hash` crate providing MD5/SHA1/SHA256/SHA512 (`bagger-hash` v0.1.0-beta.8)
- **install:** isolated installs from manifest URLs or local `.json` files (without a bucket); `-g/--global` for all-users install (admin-gated); `--arch` overrides target architecture
- **import:** real install flow (`import [FILE] [-y]` installs exported apps and restores holds); skips isolated packages from a prior export
- **checkver:** `regex` (+`reverse`/`replace`/`.NET`-style captures), `jsonpath`, `xpath` (`roxmltree`-backed subset), `script`, arch-specific specs, `github` shorthand, and `useragent` honored
- **autofetch:** `bagger autofetch <app>|all [--write]` previews expanded autoupdate URLs and hash rewrites (download / extract / json / xpath / rdf / fosshub / sourceforge / github / metalink modes)
- **virustotal:** real v3 `GET /files/{sha256}` file-report lookup with malicious/suspicious/harmless/undetected verdicts
- **download:** `aria2c` used when `aria2-enabled` is set and the binary is on PATH (honors split/max-connection/min-split/retry-wait/cookie/proxy/extra options), libcurl fallback, optional missing-binary warning (`aria2_warning_enabled`)
- **manifest-cache:** opt-in SQLite cache (`use_sqlite_cache`); raw JSON keyed by (bucket, name) with mtime+size invalidation, shared across query threads
- **config:** `show_manifest` displays manifests before install/upgrade confirmation; `aria2-*` settings with full config getters; `use_sqlite_cache`, `ignore_running_processes`, `use_external_7zip`, `use_isolated_path`, `alias` (JSON map), `gh_token`, `private_hosts`, `proxy`, `scoop_branch` (parsed)
- **arch:** process-wide architecture override via `--arch` flag or `SCOOP_ARCH` env; `install.json` records the resolved architecture
- **checkup:** broken-shim detection for content shims via `shim::target_of` (quoted-target parsing)
- **reset:** junction-aware `current` repointing (avoids os error 183 on re-extraction)
- **shim:** asymmetric add/remove now matches exactly (`{name}.exe`/`.cmd`/`.ps1`/bare); `which` resolves through `current` junctions
- **docs:** README rewritten for `bagger` with the real 31-command list and install instructions; `scripts/install.ps1` (Scoop-style one-liner) added

### Bug Fixes

- **download:** remove dead `--version` long flag (`-v` was colliding with the global `--verbose` verb)
- **network:** `get_content_length` removed; unknown remote sizes no longer validate missing cache (the `0 == 0` bug) nor skip downloads
- **query:** restored `query_installed` non-empty result (a clippy refactor had moved `return Some` into the upgradable-only branch, emptying all installed queries)
- **fs:** zero panics on hostile filesystems (non-UTF8 names, stray files, missing cache)
- **import:** `export`/`import` roundtrip now records and restores architecture per app
- **offline:** cache-miss now reports the missing cache file via `InvalidCacheFile` instead of raw os error 2

### Performance Improvements

- Parsed bucket manifests are cached and shared across query threads (avoids repeated JSON parse on `search`/`info`/`list`)
- Fat-LTO release build (`panic = "abort"`): 8.1 MB binary

---

## [0.1.0-beta.7](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.6...v0.1.0-beta.7) (2024-12-10)


### ⚠ BREAKING CHANGES

* **libscoop:** switch to use `tracing` for logging
* **libscoop:** `manifest.hash()` return type changed from `str` to `HashString`

### Features

* **hok:** add `hok completions` command to generate shell completion ([da1b6d8](https://github.com/chawyehsu/hok/commit/da1b6d8f409d8c7894872dab84e28cb8d1814fab))
* **hok:** add global `--verbose` flag ([5fd0505](https://github.com/chawyehsu/hok/commit/5fd050584e80687452a6dde798824cd312e1b74a))


### Bug Fixes

* **libscoop:** hash checking should be case insensitive (fix [#18](https://github.com/chawyehsu/hok/issues/18)) ([b3afbef](https://github.com/chawyehsu/hok/commit/b3afbef0fd438844af786fa2484fb314d7da0227))


### Code Refactoring

* **libscoop:** switch to use `tracing` for logging ([d835c7f](https://github.com/chawyehsu/hok/commit/d835c7fb96db2e99ff1b726bb3e1e5f68b31c2f7))

## [0.1.0-beta.6](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.5...v0.1.0-beta.6) (2024-10-10)


### Features

* **libscoop:** Adpot new cache filename format ([15172a9](https://github.com/chawyehsu/hok/commit/15172a9f7ac35963d1f274e51a4a72de478546c1))


### Bug Fixes

* cargo clippy fix ([71a7596](https://github.com/chawyehsu/hok/commit/71a759605a5ea2b68093cf19f1f3fc8bfe3c15b8))
* **hok:** Check if `cat_style` is empty ([4ed662e](https://github.com/chawyehsu/hok/commit/4ed662ed8bc5d35e3570a8daf1bb6fd92c429f40)), closes [#10](https://github.com/chawyehsu/hok/issues/10)
* **hok:** Remove `type.exe` dep in `hok cat` ([47a42c5](https://github.com/chawyehsu/hok/commit/47a42c57c17276c7024baaf13479b88e4eac81a1))

## [0.1.0-beta.5](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.4...v0.1.0-beta.5) (2024-07-09)


### Features

* **libscoop|config:** support `use_isolated_path` config ([1bb5ee7](https://github.com/chawyehsu/hok/commit/1bb5ee773867c490af8e21885acc87e84a33f40c))
* **libscoop:** remove env paths under isolated_path mode correctly ([2f58173](https://github.com/chawyehsu/hok/commit/2f5817387006a9334f5a74bc7c17d7063e529108))
* **libscoop:** support `use_sqlite_cache` config ([35c9577](https://github.com/chawyehsu/hok/commit/35c9577be0bf497e23c5857e350b7b5717b35645))


### Bug Fixes

* **libscoop|config:** support named `use_isolated_path` ([5e9a181](https://github.com/chawyehsu/hok/commit/5e9a18135ea309248e657c44bcf43a235833a7cf))
* **libscoop:** case insensitive match on package querying ([5efde66](https://github.com/chawyehsu/hok/commit/5efde6661999bd6c7f6bff8d1d9e30d78db5564e))
* **libscoop:** updated Config struct ([17e474a](https://github.com/chawyehsu/hok/commit/17e474a48a03f3c5281af203802a26f79e284e95))

## [0.1.0-beta.4](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.3...v0.1.0-beta.4) (2023-09-09)


### Features

* **hok:** added s shortcut for search command ([50c0bfc](https://github.com/chawyehsu/hok/commit/50c0bfcd6dd928dc105a4ec7afefb1d4e0aa97c7))


### Bug Fixes

* **hok:** added long format arg of listing known buckets ([658ef7d](https://github.com/chawyehsu/hok/commit/658ef7d9e799301bbd5807a195dd2f263933d5c1))
* **hok:** fix 50c0bfc ([387fd66](https://github.com/chawyehsu/hok/commit/387fd66637d7e53d167a18be8a0fc9daf121475e))
* **hok:** trim yes_no prompt input ([0a01f1e](https://github.com/chawyehsu/hok/commit/0a01f1e1ee65e50f9cc5e081d8daa734ea7770e4))
* **libscoop|config:** default config path should be always returned ([d3040ad](https://github.com/chawyehsu/hok/commit/d3040adf732839bb0070f6585be5428ad0d25e73))
* **libscoop|fs:** improve symlink removal logic ([398ef27](https://github.com/chawyehsu/hok/commit/398ef27fc280ded401e0e5fb5a9123d5a165b2af))
* **libscoop|resolve:** correct pinned dependency cascade resolving ([660d3e2](https://github.com/chawyehsu/hok/commit/660d3e2da5bbe5218c45c8706282bfdbc2bfe760))


### Performance Improvements

* **libscoop|manifest:** defer hash validation ([d1ff3f6](https://github.com/chawyehsu/hok/commit/d1ff3f61a46b930771b0d4809fcf77ada2ac04c3))

## [0.1.0-beta.3](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.2...v0.1.0-beta.3) (2023-08-09)


### ⚠ BREAKING CHANGES

* **libscoop|config:** `Package::manifest_path` is replaced by `manifest().path()`.

### Features

* **hok:** reflect basic support of uninstalling packages ([183cfd8](https://github.com/chawyehsu/hok/commit/183cfd8b54e8e96ce2e575240f3b7edb3183f005))
* **libscoop|sync:** basic support of uninstalling packages ([b1f0f6b](https://github.com/chawyehsu/hok/commit/b1f0f6bd3c7ee61b846d60a70889c4033730b10a))


### Bug Fixes

* **hok:** print ending newline for error report ([d1f5682](https://github.com/chawyehsu/hok/commit/d1f56822a1db93cf566265b3ec44082794896422))
* **libscoop|config:** correct `no_junction` field ([4bae700](https://github.com/chawyehsu/hok/commit/4bae700efa06b4d07506370e2eaace04ef747d3d))
* **libscoop|query:** don't create empty apps dir ([7287bd5](https://github.com/chawyehsu/hok/commit/7287bd5672f6eb88ebb52acb928bfbcc6e87877a))
* **libscoop:** added portability on non-windows ([3d1ffee](https://github.com/chawyehsu/hok/commit/3d1ffeeb39074a8c31cbb97891c082fd2a31a7fc))
* **libscoop:** avoid forcing doc target as it will fail to build ([7674f8a](https://github.com/chawyehsu/hok/commit/7674f8aef2f1cdf952c96aed6f16dbb08f65f335))
* **libscoop:** emit BucketUpdateDone event despite zero bucket ([dc4bdca](https://github.com/chawyehsu/hok/commit/dc4bdca39bec812073ca68f332d568e391736ef8))
* **libscoop:** ensure cache dir exist before downloading ([485255e](https://github.com/chawyehsu/hok/commit/485255e926df58efd6e03881d430c8496c9a4adb))
* **scoop-hash:** remove docsrs target ([b6ddd19](https://github.com/chawyehsu/hok/commit/b6ddd19f7c1f70754c70c3b1c6ca87c43e0e0754))

## [0.1.0-beta.2](https://github.com/chawyehsu/hok/compare/v0.1.0-beta.1...v0.1.0-beta.2) (2023-08-03)


### ⚠ BREAKING CHANGES

* **libscoop:** `SyncOption::NoDownloadSize` becomes `SyncOption::Offline`

### Features

* **hok|cat:** show manifest path ([7e06467](https://github.com/chawyehsu/hok/commit/7e064672ebd6aa2009f1db49ea6a0f8704139be3))
* **hok:** show bucket manifest count ([d71e193](https://github.com/chawyehsu/hok/commit/d71e193be2cc20598e53b08947635f67a1409399))
* **libscoop|download:** support injecting cookie defined in manifest ([aec7fdc](https://github.com/chawyehsu/hok/commit/aec7fdc851aee1673170182f7d382a069d514649))
* **libscoop|download:** write to temp file in downloading ([d79e598](https://github.com/chawyehsu/hok/commit/d79e5989aa01b1d49cc02003692e2f4b46991ca0))
* **libscoop|event:** added integrity check event and error type ([888afbb](https://github.com/chawyehsu/hok/commit/888afbba203b80dfd4accf57fbc99dc1b348d3e3))
* **libscoop|manifest:** impl Display for License ([e91ff0e](https://github.com/chawyehsu/hok/commit/e91ff0ec48a295a91d771e7256e542e9cab74846))
* **libscoop|resolve:** allow to select installed candidate ([8fb0ec3](https://github.com/chawyehsu/hok/commit/8fb0ec39509128498be1bcbeb3fcddb5edb16838))
* **libscoop|sync:** added SyncOption::EscapeHold for package remove ([ca8fad7](https://github.com/chawyehsu/hok/commit/ca8fad7ffbd1dd1cb0a6d1e03f924e63c5db3364))
* **libscoop:** added package integrity check logic ([57869f7](https://github.com/chawyehsu/hok/commit/57869f763e5a1a9c3668b3028d46787e5ce0e04d))
* **libscoop:** scoop-hash features passthrough ([cb027ce](https://github.com/chawyehsu/hok/commit/cb027cedd98de15aa17602234b824b240c2fcc2c))
* **scoop-hash:** support switching hashing backend ([d38658e](https://github.com/chawyehsu/hok/commit/d38658ef8785df92189b29df7094dadfc609e14c))
* **scoop-hash:** use builder pattern ([87ca347](https://github.com/chawyehsu/hok/commit/87ca3475bd4d5cb947c4ee2702807f944d92c729))


### Bug Fixes

* **hok|list:** only print upgradable when the flag is used ([558d9d3](https://github.com/chawyehsu/hok/commit/558d9d39603d85657986da43f8c98372ac938e30))
* **hok:** accumulate downloaded bytes properly ([d6fabc8](https://github.com/chawyehsu/hok/commit/d6fabc89aa0bfa1428328bddc248c11fe2e9d8e9))
* **libscoop:** package resolving is infallible when OnlyUpgrade is used ([cabd52b](https://github.com/chawyehsu/hok/commit/cabd52bdb1659bb835ad60d9074b8bbdaf345ad0))
* **libscoop:** set install state for package's upgradable reference ([63a54f7](https://github.com/chawyehsu/hok/commit/63a54f7a36ac1cdcb612437d05c752a97ed9a9e3))
* **libscoop:** use upgradable package reference when available ([9dfd93f](https://github.com/chawyehsu/hok/commit/9dfd93fcf58980a113bec1eb781f414c30489de9))


### Performance Improvements

* **libscoop:** 5x speedup on package querying ([90a8815](https://github.com/chawyehsu/hok/commit/90a881550df4c3196cd185ab34e4621f854a41b7))

## [0.1.0-beta.1](https://github.com/chawyehsu/hok/compare/v0.1.0-alpha.3...v0.1.0-beta.1) (2023-07-30)


### ⚠ BREAKING CHANGES

* **libscoop:** Some `Event` variants related to bucekt update progress have been updated to fit the latest codebase.

### Features

* **hok:** support resolving and downloading packages ([bdc08dd](https://github.com/chawyehsu/hok/commit/bdc08dd63898f7af22fa538f20b3fb068e87c26f))
* **libscoop|config:** support `SCOOP_CACHE` and `SCOOP_GLOBAL` envs ([cf2a2a5](https://github.com/chawyehsu/hok/commit/cf2a2a5503c93e5d57b5ac72aec490e2d53b2a7d))
* **libscoop|resolve:** added `resolve_cascade` ([0aa0c52](https://github.com/chawyehsu/hok/commit/0aa0c52802ea2238a31352e9ae0b19c730b7510e))
* **libscoop:** added coordination between `AssumeYes` and `NoDownloadSize` ([5e9d578](https://github.com/chawyehsu/hok/commit/5e9d5784f62fd0eb64009aa23d6d76847c164f46))
* **libscoop:** added support for package resolution and download ([4ff0d95](https://github.com/chawyehsu/hok/commit/4ff0d9573794c003c440477656e808bd527377a2))
* move to v0.1.0-beta.1 ([e1a2376](https://github.com/chawyehsu/hok/commit/e1a2376e58eb91889d7b102aaa6c415cf7b49ef1))


### Bug Fixes

* **libscoop:** ensure ops working dir exist ([0520ae8](https://github.com/chawyehsu/hok/commit/0520ae8fc6e7e560e343a4dffa4c7b514adf92c3))
* **libscoop:** handle wildcard query in upgrade operation ([639e8c6](https://github.com/chawyehsu/hok/commit/639e8c6680f53c34fbd989fdb60a0ea5e9b92c14))
* **libscoop:** update crate categories metadata ([8d6271d](https://github.com/chawyehsu/hok/commit/8d6271d208c40faf2de32787fe9c5ccf32e303f6))
* **libscoop:** update doc comments ([bcd29b4](https://github.com/chawyehsu/hok/commit/bcd29b4b172f7adf5511de457f13ce74ac676370))

## [0.1.0-alpha.3](https://github.com/chawyehsu/hok/compare/v0.1.0-alpha.2...v0.1.0-alpha.3) (2023-07-25)


### ⚠ BREAKING CHANGES

* **libscoop:** `Session::new()` is now infallible.

### Features

* **hok|config:** config-list shows the path ([679c177](https://github.com/chawyehsu/hok/commit/679c1771c036982941bce62e6db55e9098b4e739))
* **libscoop:** impl Default for Session ([d91177a](https://github.com/chawyehsu/hok/commit/d91177a269698b8fbd7b530f0100da82d4ce8879))
* **libscoop:** support loading config from all possible location ([2bcc649](https://github.com/chawyehsu/hok/commit/2bcc649808e8238bef5795c73eab41c182cac61b))
* move to v0.1.0-alpha.3 ([1ecd0ed](https://github.com/chawyehsu/hok/commit/1ecd0edf100ea4a3676494b40b5c72c787ad5501))


### Bug Fixes

* **ci:** remove unneeded condition ([718084f](https://github.com/chawyehsu/hok/commit/718084f80c615513c69a838205e58edd2a553d44))
* **libscoop|fs:** `write_json` should create file instead of dir ([54482a7](https://github.com/chawyehsu/hok/commit/54482a7c8c1733e8d0c01bac5e85fc5da7f4fd3e))
* **libscoop:** fix doctest ([c0237a2](https://github.com/chawyehsu/hok/commit/c0237a2e73d976c4f959bb0928da4cbd0ff3376e))

## [0.1.0-alpha.2](https://github.com/chawyehsu/hok/compare/v0.1.0-alpha.1...v0.1.0-alpha.2) (2023-07-25)


### ⚠ BREAKING CHANGES

* **libscoop:** APIs of operations and Session changed.
* **libscoop:** exposed modules of libscoop changed.

### Features

* **hok|cat,home:** support candidate selection ([28b56c5](https://github.com/chawyehsu/hok/commit/28b56c5ade13e1edceb04fa7c0fc7554dcc0c6a9))
* **hok:** add uninstall cmd placeholder ([c13e8be](https://github.com/chawyehsu/hok/commit/c13e8be627ab0bfb91aedfebc10ee89dc2ee8675))
* **hok:** support list held packages ([a2acb22](https://github.com/chawyehsu/hok/commit/a2acb2210bf0586f6d839d61773b1dac7d2f96f1))
* **libscoop|manifest:** support aarch64 specific fields ([639d092](https://github.com/chawyehsu/hok/commit/639d092e22dc32decc98950532614da75489dbe6))
* **libscoop|resolve:** added fn `select_candidate` ([0e296ea](https://github.com/chawyehsu/hok/commit/0e296ea5b0cb2ab884c74ccea42df86ca05840e0))
* **libscoop:** add package resolving and event bus ([434eebe](https://github.com/chawyehsu/hok/commit/434eebe3d464edb48a1d034d4e746810ba41d274))
* **libscoop:** replace ureq with libcurl ([7d3df7c](https://github.com/chawyehsu/hok/commit/7d3df7c3e954187318d46958f07d6e4b4ce9fe31))
* move to v0.1.0-alpha.2 ([24e354a](https://github.com/chawyehsu/hok/commit/24e354a7514d74878c550e25457d323e6251ee4b))


### Bug Fixes

* **hok|cat,home:** sort candidates ([c90f3f9](https://github.com/chawyehsu/hok/commit/c90f3f94367dae75cabd2dd0a562f38c924f6dbd))
* **libscoop:** dag should check self cyclic ([a5bbb0b](https://github.com/chawyehsu/hok/commit/a5bbb0bb5f57ef6d8d326e6eca9bb828f6ff6ec9))


### Miscellaneous Chores

* **libscoop:** tweak exposed modules ([f31cb64](https://github.com/chawyehsu/hok/commit/f31cb64d3794edf01b55757bb3ecdc19d4878932))

## 0.1.0-alpha.1 (2023-07-21)


### Features

* add hash crate ([aa021fb](https://github.com/chawyehsu/hok/commit/aa021fb7fa6eaa3167f803608982307ebbafe9f7))
* **api:** Introduce SPDX spec for manifest.license ([ec5e1f5](https://github.com/chawyehsu/hok/commit/ec5e1f5c6286100724f346ab55ab7fc11d02d5fe))
* **cache:** implement cache-rm ([869f095](https://github.com/chawyehsu/hok/commit/869f0956a0ccb6a8dc06d40d95bde9f79b09e504))
* **cmd:** Implement cleanup, refactor cache and list ([3ca3f26](https://github.com/chawyehsu/hok/commit/3ca3f2610ec5bf0164bfde1d4f91484423cc78c4))
* **cmd:** Implement scoop list subcommand ([0b2fdec](https://github.com/chawyehsu/hok/commit/0b2fdec835835b68b500a19d450f39e82c08a4b6))
* **cmd:** prototype of scoop home ([29c2663](https://github.com/chawyehsu/hok/commit/29c2663768e7bed616e104c6a5339b55bcdf7536))
* **cmd:** prototype of scoop info ([a207465](https://github.com/chawyehsu/hok/commit/a207465b73a704ef31014ccd408c323c45cbbdb5))
* **cmd:** prototype of scoop search (local) ([2c8e563](https://github.com/chawyehsu/hok/commit/2c8e563748539b63e6c95b9c09dbe9b1b1995199))
* **core:** add DepGraph implementation ([ca5f49f](https://github.com/chawyehsu/hok/commit/ca5f49fcd23437a5d257fd83fd23cf1c512cdb27))
* **core:** Implement update subcommand ([ad04e76](https://github.com/chawyehsu/hok/commit/ad04e76762de55954d070be3a3a352b29a78981e))
* **hash-md5:** add reset api ([3db7116](https://github.com/chawyehsu/hok/commit/3db7116412729ff2ca84de93ecd1a1850e17100e))
* **hash:** add checksum helper functions ([df24980](https://github.com/chawyehsu/hok/commit/df24980c664699b24a2efc7b609c2ba324521333))
* **hash:** add sha1 implementation ([8bee89a](https://github.com/chawyehsu/hok/commit/8bee89ae49f30cfbdb42c76c52331c7fd5ba8b82))
* **hash:** add sha256 implementation ([37f9f62](https://github.com/chawyehsu/hok/commit/37f9f622e79a5ec4d3bf122ccafd191d46041c2b))
* **hash:** add sha512 implementation ([7bbecf1](https://github.com/chawyehsu/hok/commit/7bbecf1310ee342e4d4376e413f852b16f6aadd2))
* **hash:** provided a top-level checksum api ([99fed09](https://github.com/chawyehsu/hok/commit/99fed093d48d5cf91f3db0f46f02c4d152d17043))
* Implement basic file downloads ([c5d303b](https://github.com/chawyehsu/hok/commit/c5d303bff23993ca4bc53946c074058a542a0420))
* implement hold and unhold ([682c63c](https://github.com/chawyehsu/hok/commit/682c63c78390ee4300a6c9ad42934b79be7b5866))
* implement status ([bb650d6](https://github.com/chawyehsu/hok/commit/bb650d64c711f74ff1f73c3026b86c90daafe14b))
* **scoop-cache:** implement scoop cache show ([ae018b8](https://github.com/chawyehsu/hok/commit/ae018b86a3abfe23d4f6f9c17edc9047947af8e4))
* **scoop-cache:** implement scoop cache show ([c584c90](https://github.com/chawyehsu/hok/commit/c584c90ff3e90e8744841ea64e3f732a29571b55))
* **scoop-config:** implement scoop config ([9bdc9fa](https://github.com/chawyehsu/hok/commit/9bdc9fa8a46897dea3aef636bd92d51a27b7616f))
* **search:** Add fuzzy search option ([53c8998](https://github.com/chawyehsu/hok/commit/53c8998ed98b4a150e19ffb4a10ce7a7e8ab160e))
* **search:** Implement binary search ([26ff6f2](https://github.com/chawyehsu/hok/commit/26ff6f248fc323f5be1e168e10d19fff613e07a9))
* update ([cf505f0](https://github.com/chawyehsu/hok/commit/cf505f0e51ac4c5777e260651d0ee0cd5e805abb))
* v0.1.0-alpha.1 ([f304bb2](https://github.com/chawyehsu/hok/commit/f304bb262dc1f850ae3932bb810ab91ee272fd2b))


### Bug Fixes

* **bucket-rm:** use remove_dir_all crate ([a5c9a0b](https://github.com/chawyehsu/hok/commit/a5c9a0bb309a54bae2e80552d4a5c9c0b5a4ef16))
* **core:** fix cache regex ([98f2a44](https://github.com/chawyehsu/hok/commit/98f2a44d872c876c6925e6a5ffadbc4864ddfb71))
* **core:** fix manifest download urls extraction ([7fef94c](https://github.com/chawyehsu/hok/commit/7fef94cb1235ce446d10bf0ab09bc853fc1ccd0e))
* Fix apps_in_local_bucket ([b3170c7](https://github.com/chawyehsu/hok/commit/b3170c72263dabacb027e8b66b1be3ce7113bfb7))
* fix cache rm handler ([22f51e2](https://github.com/chawyehsu/hok/commit/22f51e2cb82a80d1123895452ed2cecbf0d09b4a))
* Fix not truncating previous data when saving new configs ([ea4bf0c](https://github.com/chawyehsu/hok/commit/ea4bf0c1fa28d7ede20d35cedc5423c515a4a029))
* typo ([4ddc72f](https://github.com/chawyehsu/hok/commit/4ddc72f944d1fa235fd9644e9ec7896cf917ccc3))
* use init method to create config instance ([9aa08ba](https://github.com/chawyehsu/hok/commit/9aa08ba9caedefb62b86c4c70593463aacaeefae))


### Performance Improvements

* don't update install info if it's not held ([d8b71b1](https://github.com/chawyehsu/hok/commit/d8b71b117d97380085b14c145a59495e0ccae5f3))
* **hash-md5:** use inline fn for performance ([7f35660](https://github.com/chawyehsu/hok/commit/7f356602a93091da5b0017de3a0cb00b3a0e1bb4))
* search enhancement ([13075cc](https://github.com/chawyehsu/hok/commit/13075cc11a267d98296541fdaf3582b2f9f50eca))
