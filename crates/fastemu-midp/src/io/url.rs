//! Connection names as `Connector.open` receives them: `scheme:rest`,
//! where `rest` is `//host:port/file?query#ref` for HTTP and
//! `//host:port` for sockets and datagrams.
//!
//! Errors are messages for `IllegalArgumentException`, the exception the
//! CLDC javadoc names for a name that cannot be parsed.

use std::fmt::Write as _;

/// Splits `name` into its scheme, in lower case, and what follows the
/// colon. The scheme is letters, digits, `+`, `-` and `.`, starting with a
/// letter, as RFC 2396 says.
pub(crate) fn split_scheme(name: &str) -> Result<(String, &str), String> {
    let Some(colon) = name.find(':').filter(|&c| c > 0) else {
        return Err(format!("no ':' in the connection name {name:?}"));
    };
    let scheme = &name[..colon];
    let valid = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !valid {
        return Err(format!("invalid protocol name {scheme:?}"));
    }
    Ok((scheme.to_ascii_lowercase(), &name[colon + 1..]))
}

/// The parts of an `http:` or `https:` name that `HttpConnection`'s
/// getters return.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HttpUrl {
    /// `http` or `https`.
    pub scheme: String,
    /// The host, without brackets for an IPv6 address.
    pub host: String,
    /// The port, the scheme's default when the name gives none.
    pub port: u16,
    /// Whether the name gives the port.
    pub explicit_port: bool,
    /// The path, starting with `/`, or `None` when the name has none.
    pub file: Option<String>,
    /// The text after `?`, without it.
    pub query: Option<String>,
    /// The text after `#`, without it. It is never sent.
    pub fragment: Option<String>,
}

impl HttpUrl {
    /// Parses `rest`, the part of the name after `scheme:`.
    pub(crate) fn parse(scheme: &str, rest: &str) -> Result<Self, String> {
        let Some(after) = rest.strip_prefix("//") else {
            return Err(format!("{scheme}:{rest} does not start with {scheme}://"));
        };
        let (before_ref, fragment) = match after.split_once('#') {
            Some((b, f)) => (b, Some(f.to_owned())),
            None => (after, None),
        };
        let (before_query, query) = match before_ref.split_once('?') {
            Some((b, q)) => (b, Some(q.to_owned())),
            None => (before_ref, None),
        };
        let (authority, file) = match before_query.find('/') {
            Some(slash) => (
                &before_query[..slash],
                Some(before_query[slash..].to_owned()),
            ),
            None => (before_query, None),
        };
        // user:password@ is not part of MIDP's URLs, but a name that has
        // one still means the host after it.
        let authority = authority.rsplit('@').next().unwrap_or_default();
        let (host, port) = split_host(authority)?;
        if host.is_empty() {
            return Err(format!("no host in {scheme}:{rest}"));
        }
        let default_port = if scheme == "https" { 443 } else { 80 };
        let (port, explicit_port) = match port {
            None | Some("") => (default_port, false),
            Some(p) => (parse_port(p)?, true),
        };
        Ok(Self {
            scheme: scheme.to_owned(),
            host: host.to_owned(),
            port,
            explicit_port,
            file,
            query,
            fragment,
        })
    }

    /// The request target: the path (`/` when there is none) and the
    /// query. Bytes HTTP does not allow there (spaces, controls, text
    /// beyond ASCII) are percent-encoded, so a game that put a space in
    /// its query string still sends a valid request.
    pub(crate) fn target(&self) -> String {
        let mut raw = self.file.clone().unwrap_or_else(|| "/".to_owned());
        if let Some(q) = &self.query {
            raw.push('?');
            raw.push_str(q);
        }
        let mut out = String::with_capacity(raw.len());
        for &b in raw.as_bytes() {
            if (0x21..0x7f).contains(&b) {
                out.push(char::from(b));
            } else {
                let _ = write!(out, "%{b:02X}");
            }
        }
        out
    }

    /// The `Host` header: the host, and the port when it is not the
    /// default. IPv6 addresses get their brackets back.
    pub(crate) fn host_header(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if self.explicit_port && self.port != if self.scheme == "https" { 443 } else { 80 } {
            format!("{host}:{}", self.port)
        } else {
            host
        }
    }
}

/// Splits `host:port`, keeping a bracketed IPv6 address whole.
fn split_host(authority: &str) -> Result<(&str, Option<&str>), String> {
    if let Some(inner) = authority.strip_prefix('[') {
        let Some((host, after)) = inner.split_once(']') else {
            return Err(format!("unclosed [ in {authority:?}"));
        };
        return match after.strip_prefix(':') {
            Some(port) => Ok((host, Some(port))),
            None if after.is_empty() => Ok((host, None)),
            None => Err(format!("unexpected text after ] in {authority:?}")),
        };
    }
    Ok(match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    })
}

fn parse_port(text: &str) -> Result<u16, String> {
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid port {text:?}"));
    }
    text.parse::<u16>()
        .map_err(|_| format!("port {text} is out of range"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemes_split_at_the_first_colon_in_lower_case() {
        assert_eq!(
            split_scheme("HTTP://a/b").unwrap(),
            ("http".to_owned(), "//a/b")
        );
        assert_eq!(
            split_scheme("comm:0;baudrate=9600").unwrap(),
            ("comm".to_owned(), "0;baudrate=9600")
        );
        assert!(split_scheme("nocolon").is_err());
        assert!(split_scheme(":x").is_err());
        assert!(split_scheme("1a://x").is_err());
        assert!(split_scheme("a b://x").is_err());
    }

    #[test]
    fn http_names_give_every_part() {
        let u = HttpUrl::parse("http", "//example.com:8080/a/b.php?x=1&y=2#top").unwrap();
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 8080);
        assert_eq!(u.file.as_deref(), Some("/a/b.php"));
        assert_eq!(u.query.as_deref(), Some("x=1&y=2"));
        assert_eq!(u.fragment.as_deref(), Some("top"));
        assert_eq!(u.target(), "/a/b.php?x=1&y=2");
        assert_eq!(u.host_header(), "example.com:8080");
    }

    #[test]
    fn http_names_without_a_path_or_port_use_the_defaults() {
        let u = HttpUrl::parse("http", "//example.com").unwrap();
        assert_eq!((u.port, u.explicit_port), (80, false));
        assert_eq!(u.file, None);
        assert_eq!(u.query, None);
        assert_eq!(u.target(), "/");
        assert_eq!(u.host_header(), "example.com");
        let s = HttpUrl::parse("https", "//example.com:443?q").unwrap();
        assert_eq!(s.port, 443);
        assert_eq!(s.host_header(), "example.com");
        assert_eq!(s.target(), "/?q");
    }

    #[test]
    fn http_targets_encode_what_http_does_not_allow() {
        let u = HttpUrl::parse("http", "//h/score?name=a b&c=\u{e9}").unwrap();
        assert_eq!(u.target(), "/score?name=a%20b&c=%C3%A9");
    }

    #[test]
    fn ipv6_hosts_keep_their_brackets_in_the_host_header() {
        let u = HttpUrl::parse("http", "//[::1]:8000/").unwrap();
        assert_eq!(u.host, "::1");
        assert_eq!(u.host_header(), "[::1]:8000");
    }

    #[test]
    fn bad_http_names_are_rejected() {
        assert!(HttpUrl::parse("http", "example.com").is_err());
        assert!(HttpUrl::parse("http", "///path").is_err());
        assert!(HttpUrl::parse("http", "//h:99999/").is_err());
        assert!(HttpUrl::parse("http", "//h:8a/").is_err());
    }
}
