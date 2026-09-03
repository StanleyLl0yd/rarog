use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UrlErrorKind {
    EmptyHost,
    InvalidInternationalDomainName,
    InvalidPort,
    InvalidIpv4Address,
    InvalidIpv6Address,
    InvalidDomainCharacter,
    RelativeWithoutBase,
    RelativeWithCannotBeABaseBase,
    CannotSetHostOnCannotBeABaseUrl,
    Overflow,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlError {
    pub kind: UrlErrorKind,
    pub message: String,
}

impl UrlError {
    fn from_parse_error(error: url::ParseError) -> Self {
        let kind = match error {
            url::ParseError::EmptyHost => UrlErrorKind::EmptyHost,
            url::ParseError::IdnaError => UrlErrorKind::InvalidInternationalDomainName,
            url::ParseError::InvalidPort => UrlErrorKind::InvalidPort,
            url::ParseError::InvalidIpv4Address => UrlErrorKind::InvalidIpv4Address,
            url::ParseError::InvalidIpv6Address => UrlErrorKind::InvalidIpv6Address,
            url::ParseError::InvalidDomainCharacter => UrlErrorKind::InvalidDomainCharacter,
            url::ParseError::RelativeUrlWithoutBase => UrlErrorKind::RelativeWithoutBase,
            url::ParseError::RelativeUrlWithCannotBeABaseBase => {
                UrlErrorKind::RelativeWithCannotBeABaseBase
            }
            url::ParseError::SetHostOnCannotBeABaseUrl => {
                UrlErrorKind::CannotSetHostOnCannotBeABaseUrl
            }
            url::ParseError::Overflow => UrlErrorKind::Overflow,
            _ => UrlErrorKind::Other,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

impl fmt::Display for UrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for UrlError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UrlHost {
    Domain(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

impl fmt::Display for UrlHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(domain) => formatter.write_str(domain),
            Self::Ipv4(address) => write!(formatter, "{address}"),
            Self::Ipv6(address) => write!(formatter, "[{address}]"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WebUrl {
    inner: url::Url,
}

impl WebUrl {
    pub fn parse(input: &str) -> Result<Self, UrlError> {
        url::Url::parse(input)
            .map(|inner| Self { inner })
            .map_err(UrlError::from_parse_error)
    }

    pub fn resolve(base: &Self, input: &str) -> Result<Self, UrlError> {
        base.join(input)
    }

    pub fn join(&self, input: &str) -> Result<Self, UrlError> {
        self.inner
            .join(input)
            .map(|inner| Self { inner })
            .map_err(UrlError::from_parse_error)
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    pub fn username(&self) -> &str {
        self.inner.username()
    }

    pub fn password(&self) -> Option<&str> {
        self.inner.password()
    }

    pub fn host(&self) -> Option<UrlHost> {
        match self.inner.host()? {
            url::Host::Domain(domain) => Some(UrlHost::Domain(domain.to_owned())),
            url::Host::Ipv4(address) => Some(UrlHost::Ipv4(address)),
            url::Host::Ipv6(address) => Some(UrlHost::Ipv6(address)),
        }
    }

    pub fn port(&self) -> Option<u16> {
        self.inner.port()
    }

    pub fn port_or_known_default(&self) -> Option<u16> {
        self.inner.port_or_known_default()
    }

    pub fn path(&self) -> &str {
        self.inner.path()
    }

    pub fn query(&self) -> Option<&str> {
        self.inner.query()
    }

    pub fn fragment(&self) -> Option<&str> {
        self.inner.fragment()
    }

    pub fn cannot_be_a_base(&self) -> bool {
        self.inner.cannot_be_a_base()
    }

    pub fn without_fragment(&self) -> Self {
        let mut inner = self.inner.clone();
        inner.set_fragment(None);
        Self { inner }
    }
}

impl fmt::Display for WebUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for WebUrl {
    type Err = UrlError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_canonicalizes_special_urls() {
        let url = WebUrl::parse("HTTP://ExAmPle.com:80/a/../b?q=hello world#frag ment").unwrap();
        assert_eq!(
            url.as_str(),
            "http://example.com/b?q=hello%20world#frag%20ment"
        );
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host(), Some(UrlHost::Domain(String::from("example.com"))));
        assert_eq!(url.port(), None);
        assert_eq!(url.port_or_known_default(), Some(80));
        assert_eq!(url.path(), "/b");
        assert_eq!(url.query(), Some("q=hello%20world"));
        assert_eq!(url.fragment(), Some("frag%20ment"));
    }

    #[test]
    fn relative_references_resolve_against_a_base() {
        let base = WebUrl::parse("https://example.com/a/b/index.html").unwrap();
        let resolved = WebUrl::resolve(&base, "../image.png?x=1#top").unwrap();
        assert_eq!(
            resolved.as_str(),
            "https://example.com/a/image.png?x=1#top"
        );
    }

    #[test]
    fn international_domains_are_serialized_as_ascii() {
        let url = WebUrl::parse("https://bücher.example/").unwrap();
        assert_eq!(url.as_str(), "https://xn--bcher-kva.example/");
        assert_eq!(
            url.host(),
            Some(UrlHost::Domain(String::from("xn--bcher-kva.example")))
        );
    }

    #[test]
    fn ipv6_hosts_remain_typed_in_the_rarog_api() {
        let url = WebUrl::parse("https://[2001:db8::1]:8443/path").unwrap();
        assert_eq!(
            url.host(),
            Some(UrlHost::Ipv6("2001:db8::1".parse().unwrap()))
        );
        assert_eq!(url.port(), Some(8443));
    }

    #[test]
    fn cannot_be_a_base_urls_are_reported_without_dependency_types() {
        let url = WebUrl::parse("data:text/plain,hello").unwrap();
        assert!(url.cannot_be_a_base());
        assert_eq!(url.host(), None);
        let error = url.join("child").unwrap_err();
        assert_eq!(
            error.kind,
            UrlErrorKind::RelativeWithCannotBeABaseBase
        );
    }

    #[test]
    fn relative_input_without_a_base_has_an_owned_error() {
        let error = WebUrl::parse("../relative").unwrap_err();
        assert_eq!(error.kind, UrlErrorKind::RelativeWithoutBase);
        assert!(!error.message.is_empty());
    }

    #[test]
    fn fragment_removal_does_not_mutate_the_original_url() {
        let original = WebUrl::parse("https://example.com/path?q=1#fragment").unwrap();
        let request_url = original.without_fragment();
        assert_eq!(original.fragment(), Some("fragment"));
        assert_eq!(request_url.fragment(), None);
        assert_eq!(request_url.as_str(), "https://example.com/path?q=1");
    }
}
