use crate::{
    error::Fallible,
    event,
    internal::dag::DepGraph,
    package::{query, Package},
    Error, Session,
};

/// Resolve dependencies of the given packages.
///
/// # Note
///
/// This function ensures that packages are unique and sorted in dependency first
/// order.
pub(crate) fn resolve_dependencies(
    session: &Session,
    packages: &mut Vec<Package>,
    auto_select: bool,
) -> Fallible<()> {
    let mut graph = DepGraph::<String>::new();
    let mut to_resolve = packages.clone();

    // For performance reason, a wildcard query is done here to get all the
    // available packages in one shot and then used for the following queries.
    let synced = query::query_synced(session, &["*"], &[])?;

    loop {
        if to_resolve.is_empty() {
            break;
        }

        let mut tmp = vec![];
        tmp.append(&mut to_resolve);

        for pkg in tmp.into_iter() {
            let mut resolved = vec![];
            let deps = pkg.dependencies();

            if deps.is_empty() {
                graph.register_node(pkg.name().to_owned());
            } else {
                let queries = deps.iter().map(|d| d.as_str());

                for query in queries {
                    let mut matched = synced
                        .iter()
                        .filter(|p| {
                            let (query_bucket, query_name) =
                                query.split_once('/').unwrap_or(("", query));
                            let bucket_matched =
                                query_bucket.is_empty() || p.bucket() == query_bucket;
                            let name_matched = p.name().eq_ignore_ascii_case(query_name);
                            bucket_matched && name_matched
                        })
                        .cloned()
                        .collect::<Vec<_>>();

                    let chosen = match matched.len() {
                        0 => return Err(Error::PackageNotFound(query.to_owned())),
                        1 => matched.pop().unwrap(),
                        _ => {
                            let (installed_candidate, mut matched) = matched
                                .into_iter()
                                .partition::<Vec<_>, _>(|p| p.is_strictly_installed());

                            // There are multiple candidates for the dependency
                            // package, we need to select one from them. If a
                            // candidate is installed, it will be selected
                            // preferentially as the dependency package.
                            if !installed_candidate.is_empty() {
                                matched = installed_candidate;
                            } else {
                                select_candidate(session, &mut matched, auto_select)?;
                            }

                            matched.pop().unwrap()
                        }
                    };

                    // Record the edge even when the dep is already queued:
                    // the graph must see the full DAG, not just the
                    // frontier, or diamonds misorder (a dep queued via a
                    // longer chain still constrains its other dependents).
                    // Self-edges are meaningless and would read as cycles.
                    if chosen.name() != pkg.name() {
                        graph.register_dep(pkg.name().to_owned(), chosen.name().to_owned());
                    }
                    if !(resolved.contains(&chosen)
                        || to_resolve.contains(&chosen)
                        || packages.contains(&chosen))
                    {
                        resolved.push(chosen);
                    }
                }
            }
            // Cyclic dependency check
            graph.check()?;

            to_resolve.append(&mut resolved);
        }

        packages.extend(to_resolve.clone());
    }

    // Dependencies install before dependents, ordered by the graph (Kahn),
    // not reversed BFS: the BFS frontier order misplaces nodes whose edges
    // were already queued (e.g. installing C before D in A→B→C, A→D).
    // Ties (independent packages) break by name for determinism.
    let order = graph.walk_flatten().map_err(Error::from)?;
    let rank: std::collections::HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();
    packages.sort_by_key(|p| {
        (
            rank.get(p.name()).copied().unwrap_or(usize::MAX),
            p.name().to_owned(),
        )
    });

    Ok(())
}

/// Select one from multiple package candidates, interactively if possible.
pub(crate) fn select_candidate(
    session: &Session,
    candidates: &mut Vec<Package>,
    auto_select: bool,
) -> Fallible<()> {
    // Sort candidates by bucket priority first (known buckets in
    // `BUCKET_PRIORITY` order, unknown buckets alphabetically after),
    // then by package ident for stability.
    candidates.sort_by(|a, b| {
        bucket_priority_rank(a.bucket())
            .cmp(&bucket_priority_rank(b.bucket()))
            .then_with(|| a.ident().cmp(&b.ident()))
    });

    // With auto-select (e.g. `AssumeYes`), or when no frontend can answer,
    // take the highest-priority candidate instead of prompting.
    if auto_select {
        candidates.truncate(1);
        return Ok(());
    }

    // Only we can ask user/frontend to select one from multiple candidates
    // when the outbound tx is available for us to do an interactive q&a.
    if let Some(tx) = session.emitter() {
        let question = candidates.iter().map(|p| p.ident()).collect::<Vec<_>>();

        if tx
            .send(event::Event::PromptPackageCandidate(question))
            .is_ok()
        {
            // The unwrap is safe here because we have obtained the outbound tx,
            // so the inbound rx must be available.
            let rx = session.receiver().unwrap();

            while let Ok(answer) = rx.recv() {
                if let event::Event::PromptPackageCandidateResult(idx) = answer {
                    // bounds check
                    if idx < candidates.len() {
                        *candidates = vec![candidates[idx].clone()];

                        return Ok(());
                    }

                    return Err(Error::InvalidAnswer);
                }
            }
        }
    }

    // No frontend to ask: fall back to bucket priority instead of failing.
    candidates.truncate(1);
    Ok(())
}

/// Rank of a bucket in `BUCKET_PRIORITY`; unknown buckets sort after all
/// known ones, alphabetically (handled by the caller via ident order).
fn bucket_priority_rank(bucket: &str) -> usize {
    crate::constant::BUCKET_PRIORITY
        .iter()
        .position(|b| *b == bucket)
        .unwrap_or(usize::MAX)
}

/// Resolve unneeded dependencies of the given packages.
///
/// # Note
///
/// This function is used to resolve the unneeded dependencies of the given
/// packages. The unneeded dependencies are the dependencies that are not
/// depended by other installed packages.
///
/// The purpose is to support cascading removal of installed packages.
pub(crate) fn resolve_cascade(
    session: &Session,
    packages: &mut Vec<Package>,
    escape_hold: bool,
) -> Fallible<()> {
    let mut to_resolve = packages.clone();

    // For performance reason, a wildcard query is done here to get all the
    // installed packages in one shot and then used for the following queries.
    let installed = query::query_installed(session, &["*"], &[])?;

    loop {
        if to_resolve.is_empty() {
            break;
        }

        let tmp = to_resolve.clone();
        to_resolve = vec![];

        for pkg in tmp.into_iter() {
            // unneeded: the packages that are not depended by other installed
            // packages.
            let mut unneeded = vec![];

            let dep_names = pkg
                .dependencies()
                .into_iter()
                .map(super::extract_name)
                .collect::<Vec<_>>();

            for dep_name in dep_names {
                let mut result = installed
                    .iter()
                    .filter(|p| p.name() == dep_name)
                    .collect::<Vec<_>>();

                // The package dependency system of Scoop is not mandatory,
                // the dependency relationship is loose. For the original
                // Scoop implementation, it is allowed that a dependency may
                // be removed separately without checking its dependents.
                // This can cause the empty result of the query.
                if result.is_empty() {
                    continue;
                }

                // We queried the installed packages, it is impossible to
                // have more than one result here for an explicit package
                // name.
                assert_eq!(result.len(), 1);

                let dep_pkg = result.pop().unwrap();
                // The dependency package may be depended by other installed
                // packages.
                let mut dependents = vec![];
                installed.iter().for_each(|p| {
                    let be_dependent = p
                        .dependencies()
                        .iter()
                        .map(super::extract_name)
                        .any(|d| d == dep_pkg.name());
                    if be_dependent {
                        dependents.push(p.clone());
                    }
                });

                // `pkg` is already the package to be removed, not counted.
                dependents.retain(|p| p.name() != pkg.name());

                let needed = dependents
                    .iter()
                    .any(|p| !packages.contains(p) && !unneeded.contains(p));

                if !needed {
                    if dep_pkg.is_held() && !escape_hold {
                        return Err(Error::PackageCascadeRemoveHold(dep_pkg.name().to_owned()));
                    }
                    unneeded.push(dep_pkg.to_owned());
                }
            }

            unneeded.dedup();
            to_resolve.append(&mut unneeded);
        }

        packages.extend(to_resolve.clone());
    }

    packages.dedup();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Manifest;
    use crate::Session;

    fn test_package(name: &str, bucket: &str) -> Package {
        let manifest = Manifest::parse_bytes(
            br#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT"}"#,
            std::path::Path::new("candidate.json"),
        )
        .expect("fixture should parse");
        Package::from(name, bucket, manifest)
    }

    #[test]
    fn auto_select_prefers_bucket_priority() {
        let session = Session::new();
        let mut candidates = vec![
            test_package("app", "zzz"),
            test_package("app", "extras"),
            test_package("app", "main"),
        ];
        select_candidate(&session, &mut candidates, true).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].ident(), "main/app");
    }

    #[test]
    fn auto_select_orders_unknown_buckets_alphabetically() {
        let session = Session::new();
        let mut candidates = vec![test_package("app", "zzz"), test_package("app", "aaa")];
        select_candidate(&session, &mut candidates, true).unwrap();
        assert_eq!(candidates[0].ident(), "aaa/app");
    }

    #[test]
    fn no_frontend_falls_back_to_priority() {
        // A fresh session has no event bus, so no interactive prompt is
        // possible: priority selection applies without auto_select too.
        let session = Session::new();
        let mut candidates = vec![test_package("app", "extras"), test_package("app", "main")];
        select_candidate(&session, &mut candidates, false).unwrap();
        assert_eq!(candidates[0].ident(), "main/app");
    }

    /// Diamond with a long edge (A→B→C, A→D, C→D): reversed BFS installed
    /// C before D; the graph order puts every dependency first.
    #[test]
    fn diamond_orders_dependencies_first() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-resolve-diamond");
        let _ = std::fs::remove_dir_all(&base);
        let bucket = base.join("root/buckets/qbuk/bucket");
        std::fs::create_dir_all(&bucket).unwrap();
        let manifest = |version: &str, depends: &str| {
            format!(
                r#"{{"version": "{version}", "homepage": "https://example.com",
                    "license": "MIT", "depends": {depends}}}"#
            )
        };
        std::fs::write(bucket.join("a.json"), manifest("1.0", r#"["b", "d"]"#)).unwrap();
        std::fs::write(bucket.join("b.json"), manifest("1.0", r#"["c"]"#)).unwrap();
        std::fs::write(bucket.join("c.json"), manifest("1.0", r#"["d"]"#)).unwrap();
        std::fs::write(bucket.join("d.json"), manifest("1.0", r#"[]"#)).unwrap();
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();

        let synced = query::query_synced(&session, &["qbuk/a"], &[]).unwrap();
        assert_eq!(synced.len(), 1);
        let mut packages = synced;
        resolve_dependencies(&session, &mut packages, true).unwrap();
        let names: Vec<&str> = packages.iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["d", "c", "b", "a"]);

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }
}
