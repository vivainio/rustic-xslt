//! RFC 3986 reference resolution for `fn:resolve-uri`.
use crate::Diagnostic;
use regex::Regex;
use std::sync::LazyLock;

static REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:([^:/?#]+):)?(?://([^/?#]*))?([^?#]*)(?:\?([^#]*))?(?:#(.*))?$").unwrap()
});

struct Parts<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::new("FORG0002", message, "run")
}

fn parse(uri: &str) -> Result<Parts<'_>, Diagnostic> {
    let bytes = uri.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if !(bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit)) =>
            {
                return Err(invalid("invalid percent escape in URI"));
            }
            b' ' | b'<' | b'>' | b'"' | b'{' | b'}' | b'|' | b'\\' | b'^' | b'`' | 0..=0x1F => {
                return Err(invalid("invalid character in URI"));
            }
            _ => {}
        }
        i += 1;
    }
    let caps = REFERENCE
        .captures(uri)
        .ok_or_else(|| invalid("invalid URI reference"))?;
    let part = |i: usize| caps.get(i).map(|m| m.as_str());
    let scheme = part(1);
    if let Some(scheme) = scheme {
        let mut chars = scheme.chars();
        if !chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            || !chars.all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        {
            return Err(invalid("invalid URI scheme"));
        }
    }
    Ok(Parts {
        scheme,
        authority: part(2),
        path: part(3).unwrap_or(""),
        query: part(4),
        fragment: part(5),
    })
}

fn remove_dot_segments(path: &str) -> String {
    let mut input = path.to_owned();
    let mut output = String::new();
    while !input.is_empty() {
        if let Some(rest) = input
            .strip_prefix("../")
            .or_else(|| input.strip_prefix("./"))
        {
            input = rest.into();
        } else if input.starts_with("/./") {
            input = input[2..].into();
        } else if input == "/." {
            input = "/".into();
        } else if input.starts_with("/../") || input == "/.." {
            input = if input == "/.." {
                "/".into()
            } else {
                input[3..].into()
            };
            let cut = output.rfind('/').unwrap_or(0);
            output.truncate(cut);
        } else if input == "." || input == ".." {
            input.clear();
        } else {
            let start = usize::from(input.starts_with('/'));
            let end = input[start..].find('/').map_or(input.len(), |i| i + start);
            output.push_str(&input[..end]);
            input = input[end..].into();
        }
    }
    output
}

pub(crate) fn resolve(relative: &str, base: Option<&str>) -> Result<String, Diagnostic> {
    let reference = parse(relative)?;
    if reference.scheme.is_none()
        && reference.authority.is_none()
        && reference
            .path
            .split('/')
            .next()
            .is_some_and(|s| s.contains(':'))
    {
        return Err(invalid("relative URI has a colon in its first segment"));
    }
    let (scheme, authority, path, query);
    if reference.scheme.is_some() {
        (scheme, authority) = (reference.scheme, reference.authority);
        path = remove_dot_segments(reference.path);
        query = reference.query;
    } else {
        let Some(base) = base else {
            return Err(Diagnostic::new(
                "FONS0005",
                "relative URI needs a base URI",
                "run",
            ));
        };
        let base = parse(base)?;
        if base.scheme.is_none() || base.fragment.is_some() {
            return Err(invalid("base URI is not absolute"));
        }
        scheme = base.scheme;
        if reference.authority.is_some() {
            authority = reference.authority;
            path = remove_dot_segments(reference.path);
            query = reference.query;
        } else {
            authority = base.authority;
            if reference.path.is_empty() {
                path = base.path.to_owned();
                query = reference.query.or(base.query);
            } else {
                path = if reference.path.starts_with('/') {
                    remove_dot_segments(reference.path)
                } else {
                    let merged = if base.authority.is_some() && base.path.is_empty() {
                        format!("/{}", reference.path)
                    } else {
                        let cut = base.path.rfind('/').map_or(0, |i| i + 1);
                        format!("{}{}", &base.path[..cut], reference.path)
                    };
                    remove_dot_segments(&merged)
                };
                query = reference.query;
            }
        }
    }
    let mut out = String::new();
    if let Some(scheme) = scheme {
        out.push_str(scheme);
        out.push(':');
    }
    if let Some(authority) = authority {
        out.push_str("//");
        out.push_str(authority);
    }
    out.push_str(&path);
    if let Some(query) = query {
        out.push('?');
        out.push_str(query);
    }
    if let Some(fragment) = reference.fragment {
        out.push('#');
        out.push_str(fragment);
    }
    Ok(out)
}
