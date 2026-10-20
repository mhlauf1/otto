//! Managed git hooks (SPEC §4.3). These are a guardrail; credential scoping is the
//! guarantee (ADR-007).
//!
//! The scripts are POSIX `sh` with the workspace's identity baked in. Every value
//! interpolated here was validated in `workspace.rs` (no quotes, `$` or control chars),
//! and is single-quoted on top of that. They fail closed: anything they can't verify blocks.

use crate::workspace::{GithubOwner, Workspace, WorkspaceLayout};

const HEADER: &str =
    "#!/bin/sh\n# Managed by Otto. Regenerated on every launch; local edits are overwritten.\n";

fn preamble(workspace: &Workspace, layout: &WorkspaceLayout, action: &str) -> String {
    let allow: Vec<&str> = workspace
        .owner_allowlist
        .iter()
        .map(GithubOwner::as_str)
        .collect();
    format!(
        r#"{HEADER}# Workspace: {name} ({id})
set -u
OTTO_EXPECTED_WS='{id}'
OTTO_EXPECTED_EMAIL='{email}'
OTTO_ALLOWED_OWNERS=' {allow} '
OTTO_DOCTOR_STATUS='{doctor}'

fail() {{
  printf '\n  Otto blocked this {action} (%s workspace).\n  %s\n\n' "$OTTO_EXPECTED_WS" "$1" >&2
  exit 1
}}

lower() {{ printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }}

# Fail closed: these hooks only make sense inside the workspace that installed them.
[ "${{OTTO_WORKSPACE:-}}" = "$OTTO_EXPECTED_WS" ] ||
  fail "This repo's hooks belong to $OTTO_EXPECTED_WS, but this process is running as '${{OTTO_WORKSPACE:-unknown}}'."
"#,
        name = workspace.name,
        id = workspace.id,
        email = workspace.identity.git_email(),
        allow = allow.join(" "),
        doctor = layout.doctor_status().display(),
    )
}

/// `pre-commit`: the author and committer must be this workspace's email.
pub fn pre_commit(workspace: &Workspace, layout: &WorkspaceLayout) -> String {
    let mut s = preamble(workspace, layout, "commit");
    s.push_str(
        r#"
email_of() { lower "$(printf '%s' "$1" | sed -n 's/.*<\(.*\)>.*/\1/p')"; }

author=$(git var GIT_AUTHOR_IDENT) || fail "Couldn't read the commit author."
committer=$(git var GIT_COMMITTER_IDENT) || fail "Couldn't read the committer."
for who in "$author" "$committer"; do
  email=$(email_of "$who")
  [ "$email" = "$OTTO_EXPECTED_EMAIL" ] ||
    fail "The commit would be signed as <$email>, but this workspace commits as <$OTTO_EXPECTED_EMAIL>."
done
exit 0
"#,
    );
    s
}

/// `pre-push`: the destination owner must be allowlisted, every outgoing commit must
/// carry this workspace's email, and the Identity Doctor must be green.
pub fn pre_push(workspace: &Workspace, layout: &WorkspaceLayout) -> String {
    let mut s = preamble(workspace, layout, "push");
    s.push_str(
        r#"
[ "$(cat "$OTTO_DOCTOR_STATUS" 2>/dev/null)" = "ok" ] ||
  fail "Identity Doctor isn't green for this workspace. Open Otto and run it (Cmd-Shift-I)."

# Print the GitHub owner of a URL, or nothing if it isn't a GitHub URL.
owner_of() {
  u=$1
  case "$u" in
    *://*) rest=${u#*://}; auth=${rest%%/*}; path=${rest#*/}; host=${auth##*@}; host=${host%%:*} ;;
    *:*) auth=${u%%:*}; case "$auth" in */*) return 0 ;; esac; path=${u#*:}; host=${auth##*@} ;;
    *) return 0 ;;
  esac
  case "$(lower "$host")" in github.com|www.github.com) ;; *) return 0 ;; esac
  path=${path#/}
  lower "${path%%/*}"
}

# Credentials that bypass this workspace's gh login: a token or user:password in a
# URL, or a custom HTTP auth header (including one passed with `git -c`).
has_credentials() {
  case "$1" in
    *ghp_*|*github_pat_*|*gho_*|*ghu_*|*ghs_*|*ghr_*|*x-access-token*) return 0 ;;
    *://*:*@*) return 0 ;;
  esac
  return 1
}
if [ -n "$(git config --get-regexp '^http\..*extraheader$' 2>/dev/null)" ]; then
  fail "An http.extraHeader is set, which could authenticate as a different account."
fi

# Check the configured URL(s) *and* the effective one: a `url.*.insteadOf` rewrite
# must not be able to redirect a push to another owner.
remote_name=$1
push_url=$2
urls=$(git config --get-all "remote.$remote_name.pushurl" 2>/dev/null || git config --get-all "remote.$remote_name.url" 2>/dev/null || true)
github_targets=0
for url in $urls "$push_url"; do
  if has_credentials "$url"; then
    fail "The push URL contains a credential. Remove it from the remote (and revoke the token on GitHub)."
  fi
  owner=$(owner_of "$url")
  [ -n "$owner" ] || continue
  github_targets=$((github_targets + 1))
  case "$OTTO_ALLOWED_OWNERS" in
    *" $owner "*) ;;
    *) fail "This push goes to github.com/$owner, which isn't on this workspace's allowlist:$OTTO_ALLOWED_OWNERS" ;;
  esac
done
[ "$github_targets" -gt 0 ] || fail "Couldn't tell which GitHub owner '$push_url' belongs to."

# Every outgoing commit must be authored and committed as this workspace.
zero=0000000000000000000000000000000000000000
while read -r local_ref local_sha remote_ref remote_sha; do
  [ "$local_sha" = "$zero" ] && continue
  if [ "$remote_sha" = "$zero" ] || ! git cat-file -e "$remote_sha^{commit}" 2>/dev/null; then
    range="$local_sha --not --remotes=$remote_name"
  else
    range="$remote_sha..$local_sha"
  fi
  # shellcheck disable=SC2086
  emails=$(git log --format='%ae %ce' $range) || fail "Couldn't list the commits being pushed."
  for email in $emails; do
    [ "$(lower "$email")" = "$OTTO_EXPECTED_EMAIL" ] ||
      fail "$local_ref contains a commit by <$email>; this workspace only pushes work by <$OTTO_EXPECTED_EMAIL>."
  done
done
exit 0
"#,
    );
    s
}

/// `SSH_ASKPASS` helper: hands ssh this workspace's key passphrase from the macOS Keychain (ADR-006).
/// It answers only ssh's passphrase prompt for *this* workspace's key, so nothing else
/// (for example a git HTTPS password prompt) can ever receive the passphrase.
pub fn askpass(workspace: &Workspace, layout: &WorkspaceLayout) -> String {
    format!(
        r#"{HEADER}# Workspace: {name} ({id})
case "${{1:-}}" in
  "Enter passphrase for key '{key}':"*) ;;
  *) echo "Otto: refusing to answer an unexpected prompt." >&2; exit 1 ;;
esac
exec /usr/bin/security find-generic-password -s 'Otto SSH key passphrase' -a '{id}' -w
"#,
        name = workspace.name,
        id = workspace.id,
        key = layout.ssh_key().display(),
    )
}

/// The workspace's global git config (`GIT_CONFIG_GLOBAL`).
pub fn gitconfig(workspace: &Workspace, layout: &WorkspaceLayout) -> String {
    let id = &workspace.identity;
    format!(
        r#"# Managed by Otto for the {name} workspace. Regenerated on every launch.
[user]
	name = "{git_name}"
	email = {email}
[core]
	hooksPath = "{hooks}"
[credential]
	helper =
[credential "https://github.com"]
	helper =
	helper = !gh auth git-credential
[init]
	defaultBranch = main
[push]
	autoSetupRemote = true
"#,
        name = workspace.name,
        git_name = id.git_name(),
        email = id.git_email(),
        hooks = layout.hooks_dir().display(),
    )
}
