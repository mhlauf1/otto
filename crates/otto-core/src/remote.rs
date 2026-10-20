//! Understanding git remote URLs well enough to know which GitHub owner they point at.

/// The `owner/repo` a GitHub remote points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRemote {
    pub owner: String,
    pub repo: String,
}

/// Parse any common GitHub remote form. Returns `None` for non-GitHub hosts and local paths.
///
/// Handles `https://[user@]github.com/o/r[.git]`, `git@github.com:o/r.git`,
/// `ssh://git@github.com[:22]/o/r.git` and `git://github.com/o/r`.
pub fn parse_github_remote(url: &str) -> Option<GithubRemote> {
    let url = url.trim();
    let rest = if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(
            scheme.to_ascii_lowercase().as_str(),
            "https" | "http" | "ssh" | "git"
        ) {
            return None;
        }
        // Drop credentials and port: [user[:pass]@]host[:port]/path
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit('@').next()?.split(':').next()?;
        if !is_github_host(host) {
            return None;
        }
        path
    } else {
        // scp-like: [user@]host:path. A local path has no ':' before the first '/'.
        let (authority, path) = url.split_once(':')?;
        if authority.contains('/') {
            return None;
        }
        let host = authority.rsplit('@').next()?;
        if !is_github_host(host) {
            return None;
        }
        path.trim_start_matches('/')
    };

    let mut parts = rest.trim_end_matches('/').split('/');
    let owner = parts.next()?;
    let repo = parts.next()?.trim_end_matches(".git");
    if parts.next().is_some() || owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(GithubRemote {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
    })
}

/// GitHub token prefixes (classic, fine-grained, OAuth, user-to-server, server, refresh).
pub const TOKEN_PREFIXES: &[&str] = &["ghp_", "github_pat_", "gho_", "ghu_", "ghs_", "ghr_"];

/// True if `text` looks like it carries a secret: a GitHub token, or `user:password@`
/// inside a URL. Used to refuse remotes and config that would authenticate as
/// someone other than the workspace (embedded credentials bypass the gh helper).
pub fn contains_credentials(text: &str) -> bool {
    if TOKEN_PREFIXES.iter().any(|p| text.contains(p)) || text.contains("x-access-token") {
        return true;
    }
    // scheme://user:secret@host
    text.split("://").skip(1).any(|rest| {
        let authority = rest.split('/').next().unwrap_or("");
        authority
            .rsplit_once('@')
            .is_some_and(|(userinfo, _)| userinfo.contains(':'))
    })
}

fn is_github_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("github.com") || host.eq_ignore_ascii_case("www.github.com")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(url: &str) -> Option<String> {
        parse_github_remote(url).map(|r| r.owner)
    }

    #[test]
    fn parses_every_common_form() {
        for url in [
            "https://github.com/mhlauf1/otto.git",
            "https://github.com/mhlauf1/otto",
            "https://github.com/mhlauf1/otto/",
            "https://x-access-token:abc@github.com/mhlauf1/otto.git",
            "http://GitHub.com/mhlauf1/otto",
            "git@github.com:mhlauf1/otto.git",
            "github.com:mhlauf1/otto.git",
            "ssh://git@github.com/mhlauf1/otto.git",
            "ssh://git@github.com:22/mhlauf1/otto.git",
            "git://github.com/mhlauf1/otto",
        ] {
            assert_eq!(owner(url).as_deref(), Some("mhlauf1"), "{url}");
        }
    }

    #[test]
    fn detects_embedded_credentials() {
        for url in [
            "https://ghp_abcdefghijklmnopqrstuvwxyz0123456789@github.com/mhlauf1/lauf.git",
            "https://x-access-token:abc@github.com/mhlauf1/otto.git",
            "https://mikelauf:hunter2@github.com/mhlauf1/otto.git",
            "https://github_pat_11ABC@github.com/a/b",
        ] {
            assert!(contains_credentials(url), "{url}");
        }
        for url in [
            "https://github.com/mhlauf1/otto.git",
            "https://mhlauf1@github.com/mhlauf1/otto.git",
            "git@github.com:mhlauf1/otto.git",
            "ssh://git@github.com:22/mhlauf1/otto.git",
        ] {
            assert!(!contains_credentials(url), "{url}");
        }
    }

    #[test]
    fn ignores_other_hosts_and_local_paths() {
        for url in [
            "https://gitlab.com/mhlauf1/otto.git",
            "https://github.com.evil.com/mhlauf1/otto.git",
            "git@gitlab.com:mhlauf1/otto.git",
            "/tmp/bare.git",
            "../bare.git",
            "file:///tmp/bare.git",
            "https://github.com/mhlauf1",
            "https://github.com/a/b/c",
        ] {
            assert_eq!(owner(url), None, "{url}");
        }
    }
}
