//! Which links a game may ask the app to open.

/// The longest link, in bytes, a game may ask the app to open.
pub const MAX_LINK_BYTES: usize = 4096;

/// Whether `url` is a web link: an `http` or `https` link, scheme in any
/// case, with a host after `//`. It holds only printable ASCII and no
/// backslash (a browser reads `http://good.com\.evil.com/` as a link to
/// `evil.com`), no user name before the host, a port, when it names one,
/// of digits that make a number from 1 to 65535, and at most
/// [`MAX_LINK_BYTES`] bytes. So the link the user reads is the page that
/// opens, and it fits the prompt that shows it.
///
/// The runtime hands the app nothing else, and the app checks again
/// with this same function, so neither relies on the other.
pub fn is_web_link(url: &str) -> bool {
    if url.len() > MAX_LINK_BYTES
        || !url.bytes().all(|b| b.is_ascii_graphic())
        || url.contains('\\')
    {
        return false;
    }
    let Some((scheme, rest)) = url.split_once(':') else {
        return false;
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return false;
    }
    let Some(after) = rest.strip_prefix("//") else {
        return false;
    };
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    authority_is_sound(&after[..end])
}

/// Whether `authority` (`host`, `host:port` or `[ipv6]:port`) names a
/// host and, if it has a port, a good one.
fn authority_is_sound(authority: &str) -> bool {
    if authority.contains('@') {
        return false;
    }
    let (host, port) = if let Some(inside) = authority.strip_prefix('[') {
        let Some((host, after)) = inside.split_once(']') else {
            return false;
        };
        match after {
            "" => (host, None),
            _ => match after.strip_prefix(':') {
                Some(port) => (host, Some(port)),
                None => return false,
            },
        }
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    !host.is_empty() && port.is_none_or(is_port)
}

/// Whether `text` is the decimal digits of a port from 1 to 65535.
/// Leading zeros are allowed, as a browser allows them.
fn is_port(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit()) && text.parse::<u16>().is_ok_and(|port| port != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_links_have_a_scheme_a_host_and_a_good_port() {
        for good in [
            "http://example.com",
            "HTTPS://example.com/a",
            "http://example.com:8080/x",
            "http://example.com:1/",
            "http://example.com:65535?q=1",
            "http://example.com:0080#top",
            "http://[::1]:8080/x",
            "http://[::1]/x",
            "https://example.com/a?b=c#d:99999",
            "https://example.com/dir:80/x",
        ] {
            assert!(is_web_link(good), "{good}");
        }
    }

    #[test]
    fn links_that_are_not_what_they_read_are_refused() {
        for bad in [
            "",
            "h",
            "https://",
            "tel:123",
            "file:///x",
            "javascript:x",
            "http:example.com",
            "http:///path",
            "http://:80/",
            "http://user@example.com/",
            "http://example.com/a b",
            "http://example.com/\n",
            "https://example.com/\u{202e}moc.elpmaxe",
            "https://ex\u{0430}mple.com/",
        ] {
            assert!(!is_web_link(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_backslash_anywhere_is_refused() {
        for bad in [
            "http://good.com\\.evil.com/",
            "http://good.com\\@evil.com/",
            "http://example.com/a\\b",
            "http://example.com/?q=\\",
            "http://example.com/#\\",
            "http:\\\\example.com/",
        ] {
            assert!(!is_web_link(bad), "{bad}");
        }
    }

    #[test]
    fn a_link_is_at_most_4096_bytes() {
        let link = |bytes: usize| format!("http://example.com/{}", "a".repeat(bytes - 19));
        assert!(is_web_link(&link(MAX_LINK_BYTES)));
        assert!(!is_web_link(&link(MAX_LINK_BYTES + 1)));
        assert!(!is_web_link(&link(1 << 20)));
    }

    #[test]
    fn a_port_is_digits_from_1_to_65535() {
        for bad in [
            "http://example.com:99999/",
            "http://example.com:65536/",
            "http://example.com:abc/",
            "http://example.com:0/",
            "http://example.com:00/",
            "http://example.com:-1/",
            "http://example.com:+80/",
            "http://example.com:80a/",
            "http://example.com:/",
            "http://example.com:80:80/",
            "http://example.com:99999999999999999999/",
            "http://[::1]:99999/",
            "http://[::1]:/",
            "http://[::1]x/",
            "http://[::1/",
            "http://[]:80/",
        ] {
            assert!(!is_web_link(bad), "{bad}");
        }
    }
}
