//! `${…}` expansion for tab names and badge text.
//!
//! Rules (see README "Usage"):
//! - `${name}` is replaced by a built-in value (lowercase names, see [`BUILTIN_NAMES`]) or,
//!   failing that, the environment variable of that name. Unset gives nothing.
//! - `${NAME:-fallback}` uses `fallback` when the variable is unset or empty.
//! - `${NAME:+text}` uses `text` only when the variable is set and non-empty.
//! - Fallback and alternate text may themselves contain references, to a small depth.
//! - Expanded *values* are never scanned again: `${X}` inside a variable's value stays literal.
//! - `$` is special only when immediately followed by `{`, so `$5`, `$$$` and `$USER` are left
//!   alone. `$${` produces a literal `${`, so `$${NAME}` stays `${NAME}`.
//! - Anything malformed (`${`, `${1x}`, `${a b}`, an unterminated brace) is left as is. A typo in
//!   a config must never make a `cd` noisy.
//! - Environment names that look like secrets are never expanded (see [`is_secret_name`]): a
//!   config in a cloned repository could otherwise copy a credential into a window title, which
//!   other local processes and screen recordings can read.
//!
//! Control characters in values are stripped later by `terminal_app::sanitize`, like any other
//! text.

use std::{
    env,
    ffi::{CStr, OsStr},
    fs,
    path::{Path, PathBuf},
};

use crate::{config::SetConfig, terminal_app::TerminalApp};

/// What expansion needs to know about the current invocation.
pub struct Context {
    pub terminal: TerminalApp,
}

/// Built-in variables, resolved by term-boss itself so the hook, a manual `term-boss set` and
/// every shell agree. Looked up before the environment.
pub const BUILTIN_NAMES: [&str; 10] =
    ["hostname", "host", "username", "pwd", "pwd_short", "dir", "tty", "git_branch", "repo", "terminal"];

/// Nesting limit for references inside `:-` / `:+` text. Values are never re-expanded, so this
/// only bounds pathological literal text.
const MAX_DEPTH: usize = 4;

/// Expand `${…}` references in `tabname` and `badgetext`. A value that expands to nothing
/// (e.g. `badgetext = "${AWS_PROFILE}"` with no profile) is treated as unset, so the terminal
/// does not show an empty badge or `name []`.
pub fn expand_config(config: &mut SetConfig, ctx: &Context) {
    for field in [&mut config.tabname, &mut config.badgetext] {
        if let Some(text) = field.as_deref()
            && text.contains("${")
        {
            let expanded = expand(text, ctx);
            *field = if expanded.is_empty() { None } else { Some(expanded) };
        }
    }
}

/// Expand `${…}` references in one string.
pub fn expand(text: &str, ctx: &Context) -> String {
    render(text, 0, &mut |name| lookup(name, ctx))
}

/// Environment variable names in `text` that were refused because they look like secrets.
pub fn blocked_vars(text: &str, ctx: &Context) -> Vec<String> {
    let mut blocked: Vec<String> = Vec::new();
    render(text, 0, &mut |name| {
        if builtin(name, ctx).is_none() && is_secret_name(name) && !blocked.iter().any(|b| b == name) {
            blocked.push(name.to_string());
        }
        lookup(name, ctx)
    });
    blocked
}

/// Every built-in with its current value, for `term-boss show`.
pub fn builtins(ctx: &Context) -> Vec<(&'static str, Option<String>)> {
    BUILTIN_NAMES.iter().map(|name| (*name, builtin(name, ctx).flatten())).collect()
}

/// True for names that are likely to hold a credential.
pub fn is_secret_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["SECRET", "TOKEN", "PASSWORD", "PASSWD", "CREDENTIAL", "PRIVATE"].iter().any(|w| upper.contains(w))
        || upper.ends_with("_KEY")
}

fn lookup(name: &str, ctx: &Context) -> Option<String> {
    if let Some(value) = builtin(name, ctx) {
        return value;
    }
    if is_secret_name(name) {
        return None;
    }
    env::var_os(name).map(|v| v.to_string_lossy().into_owned())
}

/// `None`: not a built-in. `Some(None)`: a built-in with no value right now.
fn builtin(name: &str, ctx: &Context) -> Option<Option<String>> {
    Some(match name {
        "hostname" => hostname(),
        "host" => hostname().and_then(|h| h.split('.').next().filter(|s| !s.is_empty()).map(str::to_string)),
        "username" => username(),
        "pwd" => cwd().map(|p| p.display().to_string()),
        "pwd_short" => cwd().map(|p| pwd_short(&p, env::var_os("HOME").as_deref())),
        "dir" => cwd().map(|p| basename(&p)),
        "tty" => tty(),
        "git_branch" => git().and_then(|g| g.branch),
        "repo" => git().map(|g| g.repo),
        "terminal" => Some(format!("{:?}", ctx.terminal)),
        _ => return None,
    })
}

fn cwd() -> Option<PathBuf> {
    env::current_dir().ok()
}

fn basename(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".to_string())
}

/// `path` with a leading `home` shown as `~`, like zsh's `%~`.
fn pwd_short(path: &Path, home: Option<&OsStr>) -> String {
    if let Some(home) = home.filter(|h| !h.is_empty())
        && let Ok(rest) = path.strip_prefix(home)
    {
        return if rest.as_os_str().is_empty() { "~".to_string() } else { format!("~/{}", rest.display()) };
    }
    path.display().to_string()
}

fn lossy(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
    (!s.is_empty()).then_some(s)
}

fn hostname() -> Option<String> {
    let mut buf = [0u8; 256];
    // SAFETY: the buffer is valid for writes of its full length. On truncation gethostname may
    // omit the NUL, which `lossy` tolerates by stopping at the buffer end.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    (rc == 0).then(|| lossy(&buf)).flatten()
}

fn username() -> Option<String> {
    for var in ["USER", "LOGNAME"] {
        if let Some(v) = env::var_os(var)
            && !v.is_empty()
        {
            return Some(v.to_string_lossy().into_owned());
        }
    }
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buf = vec![0u8; 16 * 1024];
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: all pointers refer to live locals of the sizes passed; on success `result` points
    // at `pwd`, whose string fields point into `buf`, both still alive while we read them.
    let rc = unsafe { libc::getpwuid_r(libc::geteuid(), &mut pwd, buf.as_mut_ptr().cast(), buf.len(), &mut result) };
    if rc != 0 || result.is_null() || pwd.pw_name.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(pwd.pw_name) }.to_string_lossy().into_owned();
    (!name.is_empty()).then_some(name)
}

/// The terminal device of stdin, stdout or stderr, whichever is a tty first.
fn tty() -> Option<String> {
    for fd in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
        let mut buf = [0u8; 256];
        // SAFETY: the buffer is valid for writes of its full length; ttyname_r NUL-terminates on
        // success and returns non-zero otherwise.
        let rc = unsafe { libc::ttyname_r(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if rc == 0
            && let Some(name) = lossy(&buf)
        {
            return Some(name);
        }
    }
    None
}

struct Git {
    repo: String,
    branch: Option<String>,
}

/// Find the enclosing repository by walking up from the cwd. Reads `.git` (directory, or the
/// `gitdir:` pointer file used by worktrees and submodules) and `HEAD`; never spawns `git`.
fn git() -> Option<Git> {
    let cwd = cwd()?;
    for dir in cwd.ancestors() {
        let dot_git = dir.join(".git");
        let Ok(meta) = fs::metadata(&dot_git) else { continue };
        let gitdir = if meta.is_dir() {
            dot_git
        } else {
            let text = fs::read_to_string(&dot_git).ok()?;
            dir.join(text.strip_prefix("gitdir:")?.trim())
        };
        let branch = fs::read_to_string(gitdir.join("HEAD")).ok().and_then(|head| head_name(&head));
        return Some(Git { repo: basename(dir), branch });
    }
    None
}

/// Branch name from the contents of `HEAD`; a short hash when detached.
fn head_name(head: &str) -> Option<String> {
    let head = head.trim();
    if let Some(r) = head.strip_prefix("ref:") {
        let r = r.trim();
        let name = r.strip_prefix("refs/heads/").or_else(|| r.strip_prefix("refs/")).unwrap_or(r);
        return (!name.is_empty()).then(|| name.to_string());
    }
    (head.len() >= 8 && head.bytes().all(|b| b.is_ascii_hexdigit())).then(|| head[..8].to_string())
}

fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

enum Modifier<'a> {
    /// `${NAME}`
    Plain,
    /// `${NAME:-text}`
    Default(&'a str),
    /// `${NAME:+text}`
    Alternate(&'a str),
}

/// Parse a reference at the start of `text` (which begins with `${`). Returns the name, the
/// modifier and the index just past the closing brace.
fn parse_ref(text: &str) -> Option<(&str, Modifier<'_>, usize)> {
    let bytes = text.as_bytes();
    let mut i = 2;
    if !bytes.get(i).copied().is_some_and(is_name_start) {
        return None;
    }
    while bytes.get(i).copied().is_some_and(is_name_char) {
        i += 1;
    }
    let name = &text[2..i];
    match bytes.get(i) {
        Some(b'}') => Some((name, Modifier::Plain, i + 1)),
        Some(b':') if matches!(bytes.get(i + 1), Some(b'-' | b'+')) => {
            let start = i + 2;
            let close = matching_brace(text, start)?;
            let body = &text[start..close];
            let modifier = if bytes[i + 1] == b'+' { Modifier::Alternate(body) } else { Modifier::Default(body) };
            Some((name, modifier, close + 1))
        }
        _ => None,
    }
}

/// Index of the `}` that closes the body starting at `start`, skipping nested `${…}`.
fn matching_brace(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'$' if bytes.get(i + 1) == Some(&b'{') => {
                depth += 1;
                i += 2;
                continue;
            }
            b'}' if depth == 0 => return Some(i),
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    None
}

/// Walk `text`, calling `resolve(name)` for each well-formed reference. `None` or an empty
/// value counts as unset, matching the shell's `:-` and `:+`.
fn render(text: &str, depth: usize, resolve: &mut dyn FnMut(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        if let Some(after) = tail.strip_prefix("$${") {
            // Escaped: emit a literal `${`; what follows is ordinary text.
            out.push_str("${");
            rest = after;
            continue;
        }
        if tail.starts_with("${")
            && let Some((name, modifier, end)) = parse_ref(tail)
        {
            let value = resolve(name).filter(|v| !v.is_empty());
            match (value, modifier) {
                (Some(v), Modifier::Plain | Modifier::Default(_)) => out.push_str(&v),
                (Some(_), Modifier::Alternate(body)) | (None, Modifier::Default(body)) => {
                    out.push_str(&nested(body, depth, resolve));
                }
                (None, Modifier::Plain | Modifier::Alternate(_)) => {}
            }
            rest = &tail[end..];
            continue;
        }
        out.push('$');
        rest = &tail[1..];
    }
    out.push_str(rest);
    out
}

fn nested(body: &str, depth: usize, resolve: &mut dyn FnMut(&str) -> Option<String>) -> String {
    if depth >= MAX_DEPTH { body.to_string() } else { render(body, depth + 1, resolve) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: &str = "TERM_BOSS_TEST_SET";
    const EMPTY: &str = "TERM_BOSS_TEST_EMPTY";
    const UNSET: &str = "TERM_BOSS_TEST_UNSET";
    const REF: &str = "TERM_BOSS_TEST_REF";

    fn setup() -> Context {
        // Tests in this module only read these; no other test touches the environment.
        unsafe {
            env::set_var(SET, "val");
            env::set_var(EMPTY, "");
            env::remove_var(UNSET);
            env::set_var(REF, "${TERM_BOSS_TEST_SET}");
        }
        Context { terminal: TerminalApp::Generic }
    }

    fn x(text: &str) -> String {
        expand(text, &setup())
    }

    #[test]
    fn plain_and_unset() {
        assert_eq!(x("a ${TERM_BOSS_TEST_SET} b"), "a val b");
        assert_eq!(x("a ${TERM_BOSS_TEST_UNSET} b"), "a  b");
        assert_eq!(x("no refs"), "no refs");
        assert_eq!(x(""), "");
    }

    #[test]
    fn default_modifier() {
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-fb}"), "fb");
        assert_eq!(x("${TERM_BOSS_TEST_EMPTY:-fb}"), "fb");
        assert_eq!(x("${TERM_BOSS_TEST_SET:-fb}"), "val");
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-}"), "");
    }

    #[test]
    fn alternate_modifier() {
        assert_eq!(x("${TERM_BOSS_TEST_SET:+yes}"), "yes");
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:+yes}"), "");
        assert_eq!(x("${TERM_BOSS_TEST_EMPTY:+yes}"), "");
        assert_eq!(x("x${TERM_BOSS_TEST_UNSET:+y}z"), "xz");
    }

    #[test]
    fn nested_references() {
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-${TERM_BOSS_TEST_SET}}"), "val");
        assert_eq!(x("${TERM_BOSS_TEST_SET:+[${TERM_BOSS_TEST_SET}]}"), "[val]");
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-a${TERM_BOSS_TEST_UNSET:-b}c}"), "abc");
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-a${TERM_BOSS_TEST_UNSET:-b}c}d"), "abcd");
        // Beyond MAX_DEPTH the body is emitted as it is.
        let deep = "${U:-${U:-${U:-${U:-${U:-${TERM_BOSS_TEST_SET}}}}}}".replace("U", UNSET);
        assert_eq!(x(&deep), "${TERM_BOSS_TEST_SET}");
    }

    #[test]
    fn values_are_not_re_expanded() {
        assert_eq!(x("${TERM_BOSS_TEST_REF}"), "${TERM_BOSS_TEST_SET}");
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-${TERM_BOSS_TEST_REF}}"), "${TERM_BOSS_TEST_SET}");
    }

    #[test]
    fn dollar_is_literal_unless_followed_by_brace() {
        assert_eq!(x("cost $5 and $$$"), "cost $5 and $$$");
        assert_eq!(x("$TERM_BOSS_TEST_SET"), "$TERM_BOSS_TEST_SET");
        assert_eq!(x("trailing $"), "trailing $");
    }

    #[test]
    fn escape() {
        assert_eq!(x("$${TERM_BOSS_TEST_SET}"), "${TERM_BOSS_TEST_SET}");
        // Scanning left to right: the first `$` is not followed by `{`, so it is literal, and
        // the remaining `$${…}` is the escape.
        assert_eq!(x("$$${TERM_BOSS_TEST_SET}"), "$${TERM_BOSS_TEST_SET}");
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-$${TERM_BOSS_TEST_SET}}"), "${TERM_BOSS_TEST_SET}");
    }

    #[test]
    fn malformed_left_alone() {
        for s in [
            "${",
            "${}",
            "${1x}",
            "${a b}",
            "${TERM_BOSS_TEST_SET",
            "${TERM_BOSS_TEST_SET:fb}",
            "${TERM_BOSS_TEST_UNSET:-fb",
        ] {
            assert_eq!(x(s), s);
        }
        // An unterminated outer reference is literal, but scanning continues, so a well-formed
        // reference inside it still expands.
        assert_eq!(x("${TERM_BOSS_TEST_UNSET:-${TERM_BOSS_TEST_SET}"), "${TERM_BOSS_TEST_UNSET:-val");
    }

    #[test]
    fn secrets_are_not_expanded() {
        let ctx = setup();
        unsafe { env::set_var("TERM_BOSS_TEST_SECRET", "hunter2") };
        assert_eq!(expand("[${TERM_BOSS_TEST_SECRET}]", &ctx), "[]");
        assert_eq!(expand("[${TERM_BOSS_TEST_SECRET:-hidden}]", &ctx), "[hidden]");
        assert_eq!(
            blocked_vars("${TERM_BOSS_TEST_SECRET} ${AWS_SECRET_ACCESS_KEY} ${HOME} ${AWS_SECRET_ACCESS_KEY}", &ctx),
            vec!["TERM_BOSS_TEST_SECRET".to_string(), "AWS_SECRET_ACCESS_KEY".to_string()]
        );
        // Secrets inside modifier text are found too.
        assert_eq!(blocked_vars("${TERM_BOSS_TEST_SET:+${DB_PASSWORD}}", &ctx), vec!["DB_PASSWORD".to_string()]);
        for name in ["GITHUB_TOKEN", "DB_PASSWORD", "passwd", "GOOGLE_CREDENTIALS", "PRIVATE_THING", "API_KEY", "OPENAI_API_KEY"] {
            assert!(is_secret_name(name), "{name}");
        }
        for name in ["AWS_PROFILE", "KUBECONFIG", "HOME", "USER", "KEYBOARD", "VIRTUAL_ENV", "KEY_"] {
            assert!(!is_secret_name(name), "{name}");
        }
    }

    #[test]
    fn builtin_host_and_user() {
        let ctx = setup();
        let hostname = expand("${hostname}", &ctx);
        let host = expand("${host}", &ctx);
        assert!(!hostname.is_empty());
        assert!(!host.is_empty() && !host.contains('.') && hostname.starts_with(&host));
        if let Ok(user) = env::var("USER") {
            assert_eq!(expand("${username}", &ctx), user);
        } else {
            assert!(!expand("${username}", &ctx).is_empty());
        }
        let tty = expand("${tty}", &ctx);
        assert!(tty.is_empty() || tty.starts_with("/dev/"), "{tty}");
        assert_eq!(expand("${terminal}", &ctx), "Generic");
        assert_eq!(expand("${terminal}", &Context { terminal: TerminalApp::Iterm2 }), "Iterm2");
        assert_eq!(expand("${terminal}", &Context { terminal: TerminalApp::Ghostty }), "Ghostty");
    }

    #[test]
    fn builtin_paths_and_git() {
        let ctx = setup();
        // cargo runs tests with the crate root as cwd, which is the repository root.
        let cwd = env::current_dir().unwrap();
        let name = cwd.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(expand("${pwd}", &ctx), cwd.display().to_string());
        assert_eq!(expand("${dir}", &ctx), name);
        assert_eq!(expand("${repo}", &ctx), name);
        let head = fs::read_to_string(cwd.join(".git/HEAD")).unwrap();
        assert_eq!(expand("${git_branch}", &ctx), head_name(&head).unwrap());
        assert_eq!(expand("${repo:-${dir}}", &ctx), name);
    }

    #[test]
    fn pwd_short_replaces_home() {
        let p = Path::new("/home/me/dev/x");
        assert_eq!(pwd_short(p, Some(OsStr::new("/home/me"))), "~/dev/x");
        assert_eq!(pwd_short(Path::new("/home/me"), Some(OsStr::new("/home/me"))), "~");
        assert_eq!(pwd_short(p, Some(OsStr::new("/home/other"))), "/home/me/dev/x");
        assert_eq!(pwd_short(p, Some(OsStr::new(""))), "/home/me/dev/x");
        assert_eq!(pwd_short(p, None), "/home/me/dev/x");
    }

    #[test]
    fn head_names() {
        assert_eq!(head_name("ref: refs/heads/main\n").as_deref(), Some("main"));
        assert_eq!(head_name("ref: refs/heads/feat/x").as_deref(), Some("feat/x"));
        assert_eq!(head_name("ref: refs/remotes/origin/main").as_deref(), Some("remotes/origin/main"));
        assert_eq!(head_name("0123456789abcdef0123456789abcdef01234567\n").as_deref(), Some("01234567"));
        assert_eq!(head_name("garbage"), None);
        assert_eq!(head_name(""), None);
    }

    #[test]
    fn expand_config_only_touches_text_fields() {
        let ctx = setup();
        let mut config = SetConfig {
            tabname: Some("${TERM_BOSS_TEST_SET}".into()),
            badgetext: Some("plain".into()),
            scheme: Some("${TERM_BOSS_TEST_SET}".into()),
            ..Default::default()
        };
        expand_config(&mut config, &ctx);
        assert_eq!(config.tabname.as_deref(), Some("val"));
        assert_eq!(config.badgetext.as_deref(), Some("plain"));
        assert_eq!(config.scheme.as_deref(), Some("${TERM_BOSS_TEST_SET}"));
    }

    #[test]
    fn expanding_to_nothing_unsets_the_field() {
        let ctx = setup();
        let mut config = SetConfig {
            tabname: Some("${TERM_BOSS_TEST_UNSET}".into()),
            badgetext: Some(String::new()),
            ..Default::default()
        };
        expand_config(&mut config, &ctx);
        assert_eq!(config.tabname, None);
        // A literal empty string is left alone; only expansion results are checked.
        assert_eq!(config.badgetext.as_deref(), Some(""));
    }
}
