use super::{UrlError, UrlErrorKind, UrlHost, WebUrl};
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_OPAQUE_ORIGIN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpaqueOriginId(NonZeroU64);

impl OpaqueOriginId {
    fn allocate() -> Result<Self, UrlError> {
        let raw = NEXT_OPAQUE_ORIGIN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                UrlError::new(
                    UrlErrorKind::IdentitySpaceExhausted,
                    "opaque origin identity space is exhausted",
                )
            })?;
        NonZeroU64::new(raw).map(Self).ok_or_else(|| {
            UrlError::new(
                UrlErrorKind::IdentitySpaceExhausted,
                "opaque origin identity space is exhausted",
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    Tuple {
        scheme: String,
        host: UrlHost,
        port: u16,
    },
    Opaque(OpaqueOriginId),
}

impl Origin {
    fn new_opaque() -> Result<Self, UrlError> {
        OpaqueOriginId::allocate().map(Self::Opaque)
    }

    pub fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque(_))
    }

    pub fn ascii_serialization(&self) -> String {
        match self {
            Self::Opaque(_) => String::from("null"),
            Self::Tuple { scheme, host, port } => {
                if known_default_port(scheme) == Some(*port) {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                }
            }
        }
    }

    pub fn site(&self) -> SiteIdentity {
        match self {
            Self::Opaque(identity) => SiteIdentity::Opaque(*identity),
            Self::Tuple { scheme, host, .. } => SiteIdentity::Tuple {
                scheme: scheme.clone(),
                host: site_host(host),
            },
        }
    }

    pub fn same_origin(&self, other: &Self) -> bool {
        self == other
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.ascii_serialization())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SiteIdentity {
    Tuple { scheme: String, host: UrlHost },
    Opaque(OpaqueOriginId),
}

impl SiteIdentity {
    pub fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque(_))
    }

    pub fn ascii_serialization(&self) -> String {
        match self {
            Self::Opaque(_) => String::from("null"),
            Self::Tuple { scheme, host } => format!("{scheme}://{host}"),
        }
    }

    pub fn same_site(&self, other: &Self) -> bool {
        self == other
    }
}

impl fmt::Display for SiteIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.ascii_serialization())
    }
}

impl WebUrl {
    pub fn origin(&self) -> Result<Origin, UrlError> {
        match self.inner.origin() {
            url::Origin::Opaque(_) => Origin::new_opaque(),
            url::Origin::Tuple(scheme, host, port) => Ok(Origin::Tuple {
                scheme,
                host: owned_host(host),
                port,
            }),
        }
    }

    pub fn site_identity(&self) -> Result<SiteIdentity, UrlError> {
        self.origin().map(|origin| origin.site())
    }
}

fn owned_host(host: url::Host<String>) -> UrlHost {
    match host {
        url::Host::Domain(domain) => UrlHost::Domain(domain),
        url::Host::Ipv4(address) => UrlHost::Ipv4(address),
        url::Host::Ipv6(address) => UrlHost::Ipv6(address),
    }
}

fn site_host(host: &UrlHost) -> UrlHost {
    match host {
        UrlHost::Domain(domain) => {
            let registrable = psl::domain(domain.as_bytes())
                .and_then(|domain| std::str::from_utf8(domain.as_bytes()).ok())
                .unwrap_or(domain)
                .to_owned();
            UrlHost::Domain(registrable)
        }
        UrlHost::Ipv4(address) => UrlHost::Ipv4(*address),
        UrlHost::Ipv6(address) => UrlHost::Ipv6(*address),
    }
}

fn known_default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "ftp" => Some(21),
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_origins_include_effective_port_and_ignore_path() {
        let first = WebUrl::parse("https://example.com/a")
            .unwrap()
            .origin()
            .unwrap();
        let second = WebUrl::parse("https://example.com:443/b?q=1")
            .unwrap()
            .origin()
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.ascii_serialization(), "https://example.com");
    }

    #[test]
    fn different_ports_are_cross_origin_but_same_site() {
        let first = WebUrl::parse("https://shop.example.com:8443/")
            .unwrap()
            .origin()
            .unwrap();
        let second = WebUrl::parse("https://cdn.example.com/")
            .unwrap()
            .origin()
            .unwrap();
        assert!(!first.same_origin(&second));
        assert!(first.site().same_site(&second.site()));
        assert_eq!(first.site().ascii_serialization(), "https://example.com");
    }

    #[test]
    fn registrable_domains_define_schemeful_site() {
        let first = WebUrl::parse("https://a.example.co.uk/")
            .unwrap()
            .site_identity()
            .unwrap();
        let second = WebUrl::parse("https://b.example.co.uk/")
            .unwrap()
            .site_identity()
            .unwrap();
        let cross_scheme = WebUrl::parse("http://b.example.co.uk/")
            .unwrap()
            .site_identity()
            .unwrap();
        assert_eq!(first.ascii_serialization(), "https://example.co.uk");
        assert!(first.same_site(&second));
        assert!(!first.same_site(&cross_scheme));
    }

    #[test]
    fn private_suffixes_isolate_sites() {
        let first = WebUrl::parse("https://alice.github.io/")
            .unwrap()
            .site_identity()
            .unwrap();
        let second = WebUrl::parse("https://bob.github.io/")
            .unwrap()
            .site_identity()
            .unwrap();
        assert_eq!(first.ascii_serialization(), "https://alice.github.io");
        assert_eq!(second.ascii_serialization(), "https://bob.github.io");
        assert!(!first.same_site(&second));
    }

    #[test]
    fn ip_hosts_remain_exact_site_boundaries() {
        let first = WebUrl::parse("https://127.0.0.1:8443/")
            .unwrap()
            .site_identity()
            .unwrap();
        let second = WebUrl::parse("https://127.0.0.1/")
            .unwrap()
            .site_identity()
            .unwrap();
        let other = WebUrl::parse("https://127.0.0.2/")
            .unwrap()
            .site_identity()
            .unwrap();
        assert!(first.same_site(&second));
        assert!(!first.same_site(&other));
    }

    #[test]
    fn opaque_origins_are_unique_and_clone_preserves_identity() {
        let url = WebUrl::parse("data:text/plain,hello").unwrap();
        let first = url.origin().unwrap();
        let cloned = first.clone();
        let second = url.origin().unwrap();
        assert!(first.is_opaque());
        assert_eq!(first, cloned);
        assert_ne!(first, second);
        assert_eq!(first.ascii_serialization(), "null");
        assert_eq!(first.site().ascii_serialization(), "null");
    }

    #[test]
    fn blob_urls_inherit_the_embedded_tuple_origin() {
        let origin = WebUrl::parse("blob:https://example.com/550e8400-e29b-41d4-a716-446655440000")
            .unwrap()
            .origin()
            .unwrap();
        assert_eq!(origin.ascii_serialization(), "https://example.com");
    }
}
