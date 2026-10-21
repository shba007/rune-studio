use reqwest::Url;
use std::net::IpAddr;

use super::record::DataClass;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny(String),
}

impl PolicyDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }
}

/// Checks whether a request with an optional data class is allowed to be transmitted to the given endpoint.
///
/// Rules:
/// 1. Untagged requests and `general` class data may go to any endpoint the user has configured.
/// 2. Sensitive classes (`health`, `mind`, `finance`) may go ONLY to an endpoint that is local.
///
/// Local is decided in this order:
/// (1) Model profile `locality` (when present): `local` allows, `cloud` and `lan` deny, `unknown` denies.
/// (2) Otherwise, fallback by address: base URL host is a loopback address (`127.0.0.0/8`, `::1`, `localhost`).
///     Anything else denies. Never decide by model name.
pub fn may_send(
    class: Option<DataClass>,
    endpoint: &str,
    model_locality: Option<&str>,
) -> PolicyDecision {
    let Some(data_class) = class else {
        return PolicyDecision::Allow;
    };

    if !data_class.is_sensitive() {
        return PolicyDecision::Allow;
    }

    // Step 1: Check model profile locality if present
    if let Some(locality) = model_locality {
        match locality.trim().to_ascii_lowercase().as_str() {
            "local" => return PolicyDecision::Allow,
            "cloud" => {
                return PolicyDecision::Deny(
                    "Model locality is explicitly reported as 'cloud'".to_string(),
                );
            }
            "lan" => {
                return PolicyDecision::Deny(
                    "Model locality is explicitly reported as 'lan'".to_string(),
                );
            }
            other => {
                return PolicyDecision::Deny(format!(
                    "Model locality is not verified as local (reported: '{other}')"
                ));
            }
        }
    }

    // Step 2: Fallback by address
    match parse_and_validate_loopback_host(endpoint) {
        Ok(true) => PolicyDecision::Allow,
        Ok(false) => PolicyDecision::Deny(format!(
            "Endpoint '{endpoint}' does not resolve to a local loopback address (127.0.0.0/8, ::1, or localhost)"
        )),
        Err(err) => PolicyDecision::Deny(format!("Invalid endpoint URL: {err}")),
    }
}

/// Tagged non-chat routes lack authoritative response attribution today.
/// They require a loopback endpoint as well as the normal model-locality check.
pub fn may_send_to_route(
    class: Option<DataClass>,
    endpoint: &str,
    model_locality: Option<&str>,
    chat: bool,
) -> PolicyDecision {
    if class.is_some() && !chat && !parse_and_validate_loopback_host(endpoint).unwrap_or(false) {
        return PolicyDecision::Deny("Tagged non-chat requests require a loopback endpoint until attribution headers are available".into());
    }
    may_send(class, endpoint, model_locality)
}

/// The canonical header wins over any legacy aliases, including for a local reply.
pub fn response_is_cloud(headers: &reqwest::header::HeaderMap) -> bool {
    for name in ["x-model-locality", "x-rune-locality", "x-infer-locality"] {
        if let Some(value) = headers.get(name) {
            return value
                .to_str()
                .map(|s| {
                    s.trim().eq_ignore_ascii_case("cloud") || s.trim().eq_ignore_ascii_case("lan")
                })
                .unwrap_or(true);
        }
    }
    false
}

/// Robust URL host parser that strictly prevents substring attacks, userinfo injection,
/// port tricks, and non-loopback domains.
pub fn parse_and_validate_loopback_host(endpoint: &str) -> Result<bool, String> {
    let url = Url::parse(endpoint).map_err(|e| e.to_string())?;

    if !matches!(url.scheme(), "http" | "https") {
        return Ok(false);
    }
    let Some(host_str) = url.host_str() else {
        return Ok(false);
    };

    // 1. Check localhost domain (RFC 6761)
    let host_lower = host_str.to_ascii_lowercase();
    if host_lower == "localhost" {
        return Ok(true);
    }

    // 2. Check IP addresses
    if let Ok(ip) = host_str.parse::<IpAddr>() {
        return Ok(is_loopback_ip(&ip));
    }

    // Handle bracketed IPv6 if not stripped by url
    let unbracketed = host_str.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = unbracketed.parse::<IpAddr>() {
        return Ok(is_loopback_ip(&ip));
    }

    Ok(false)
}

fn is_loopback_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(), // 127.0.0.0/8
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                return true;
            }
            // Check for IPv4-mapped IPv6 loopback (::ffff:127.0.0.1)
            if let Some(v4) = v6.to_ipv4_mapped() {
                return v4.is_loopback();
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_class_can_send_anywhere() {
        assert_eq!(
            may_send(
                Some(DataClass::General),
                "https://api.openai.com/v1",
                Some("cloud")
            ),
            PolicyDecision::Allow
        );
        assert_eq!(
            may_send(Some(DataClass::General), "http://192.168.1.100:8080", None),
            PolicyDecision::Allow
        );
        assert_eq!(
            may_send(None, "https://api.anthropic.com/v1", Some("cloud")),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn sensitive_class_honors_model_locality() {
        // Local allows
        assert_eq!(
            may_send(
                Some(DataClass::Finance),
                "http://127.0.0.1:8080",
                Some("local")
            ),
            PolicyDecision::Allow
        );
        // Cloud denies
        assert!(matches!(
            may_send(
                Some(DataClass::Health),
                "http://127.0.0.1:8080",
                Some("cloud")
            ),
            PolicyDecision::Deny(_)
        ));
        // Lan denies
        assert!(matches!(
            may_send(Some(DataClass::Mind), "http://127.0.0.1:8080", Some("lan")),
            PolicyDecision::Deny(_)
        ));
        // Unknown denies
        assert!(matches!(
            may_send(
                Some(DataClass::Finance),
                "http://127.0.0.1:8080",
                Some("unknown")
            ),
            PolicyDecision::Deny(_)
        ));
    }

    #[test]
    fn sensitive_class_address_fallback_loopback_allowed() {
        let valid_endpoints = [
            "http://127.0.0.1:8080",
            "http://127.0.0.1",
            "http://127.0.0.5:11434",
            "http://127.255.255.254:9999",
            "http://localhost:8080",
            "http://localhost",
            "http://[::1]:8080",
            "http://[::1]",
            "http://[::ffff:127.0.0.1]:8080",
        ];

        for ep in valid_endpoints {
            assert_eq!(
                may_send(Some(DataClass::Health), ep, None),
                PolicyDecision::Allow,
                "Endpoint {ep} should be allowed as loopback"
            );
        }
    }

    #[test]
    fn sensitive_class_address_fallback_attacks_and_non_loopback_denied() {
        let denied_endpoints = [
            "http://test.localhost:8080",
            "ftp://localhost",
            // Substring attacks
            "http://127.0.0.1.evil.com",
            "http://127.0.0.1.attacker.net:8080",
            "http://localhost.evil.com",
            // Userinfo tricks
            "http://localhost@evil.com",
            "http://127.0.0.1:password@attacker.org:8080",
            "http://localhost:8080@attacker.org",
            // Port number trick
            "http://evil.com:12701",
            // LAN and private networks
            "http://192.168.1.1:8080",
            "http://10.0.0.1:8080",
            "http://172.16.0.1:8080",
            // Public cloud IP
            "http://93.184.216.34:8080",
            // IPv6 non-loopback
            "http://[2001:db8::1]:8080",
            "http://[fe80::1]:8080",
            // Cloud domains
            "https://api.openai.com/v1",
            "https://api.together.xyz/v1",
            // Malformed
            "not a url",
            "http://",
        ];

        for ep in denied_endpoints {
            assert!(
                matches!(
                    may_send(Some(DataClass::Finance), ep, None),
                    PolicyDecision::Deny(_)
                ),
                "Endpoint {ep} must be DENIED for sensitive classes"
            );
        }
    }
    #[test]
    fn tagged_non_chat_routes_require_loopback_even_for_local_models() {
        for class in [DataClass::General, DataClass::Health] {
            assert!(
                !may_send_to_route(Some(class), "https://remote.example", Some("local"), false)
                    .is_allowed()
            );
            assert!(
                may_send_to_route(Some(class), "http://127.0.0.1:8080", Some("local"), false)
                    .is_allowed()
            );
        }
    }

    #[test]
    fn canonical_response_locality_wins() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-model-locality", "cloud".parse().unwrap());
        headers.insert("x-rune-locality", "local".parse().unwrap());
        assert!(response_is_cloud(&headers));
        headers.insert("x-model-locality", "local".parse().unwrap());
        headers.insert("x-rune-locality", "cloud".parse().unwrap());
        assert!(!response_is_cloud(&headers));
    }
}
