//! Single local user, no identity: the dev server binds loopback only, so the
//! peer's address is the trust boundary. Even if it is reached otherwise,
//! non-loopback peers are disconnected without a reply; loopback requests are
//! served without cookies.

use std::net::IpAddr;

use rustex1_devserver::{Gate, GateReply, GateVerdict, Request};

pub struct GateImpl;

/// Identity/auth paths Caper never exposes; 404 instead of SPA fallback.
/// Unknown `/api/*` routes already 404 in `handle_api`.
const NOT_EXPOSED: &[&str] = &[
    "/login",
    "/api/me",
    "/api/users",
    "/api/admin",
    "/masquerade",
    "/api/masquerade",
    "/logout",
    "/api/logout",
    "/share",
    "/api/share",
    "/api/shares",
];

fn is_loopback(peer: Option<IpAddr>) -> bool {
    peer.map(|p| p.is_loopback()).unwrap_or(false)
}

impl Gate for GateImpl {
    fn early(&self, _req: &Request, peer: Option<IpAddr>) -> bool {
        is_loopback(peer)
    }

    fn handle(&self, req: &Request, peer: Option<IpAddr>) -> GateVerdict {
        if !self.early(req, peer) {
            return GateVerdict::HangUp;
        }
        if NOT_EXPOSED.iter().any(|blocked| {
            req.path.strip_prefix(blocked).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        }) {
            // No identity/auth routes exist; reject instead of serving the SPA fallback.
            return GateVerdict::Reply(GateReply {
                status: 404,
                headers: vec![],
                content_type: "text/plain",
                body: b"not found".to_vec(),
            });
        }
        GateVerdict::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn loopback_request(path: &str) -> Request {
        Request {
            path: path.into(),
            ..Default::default()
        }
    }

    #[test]
    fn login_is_rejected_without_spa_fallback() {
        let peer = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
        match GateImpl.handle(&loopback_request("/login"), peer) {
            GateVerdict::Reply(reply) => assert_eq!(reply.status, 404),
            _ => panic!("expected /login to be rejected"),
        }
    }

    #[test]
    fn identity_paths_are_not_exposed() {
        let peer = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
        for path in NOT_EXPOSED {
            match GateImpl.handle(&loopback_request(path), peer) {
                GateVerdict::Reply(reply) => assert_eq!(reply.status, 404, "{path}"),
                _ => panic!("expected {path} to be rejected"),
            }
        }
        match GateImpl.handle(&loopback_request("/share/session-id"), peer) {
            GateVerdict::Reply(reply) => assert_eq!(reply.status, 404),
            _ => panic!("expected /share/* to be rejected"),
        }
    }

    #[test]
    fn non_loopback_peers_are_disconnected_without_a_reply() {
        let peer = Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)));
        assert!(!GateImpl.early(&loopback_request("/"), peer));
        assert!(matches!(GateImpl.handle(&loopback_request("/"), peer), GateVerdict::HangUp));
        assert!(matches!(GateImpl.handle(&loopback_request("/api/me"), peer), GateVerdict::HangUp));
        assert!(!GateImpl.early(&loopback_request("/"), None));
    }

    #[test]
    fn loopback_is_served_without_cookies() {
        let peer = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
        // `loopback_request` carries no headers; no cookie is required.
        match GateImpl.handle(&loopback_request("/api/sessions"), peer) {
            GateVerdict::Allow => {}
            _ => panic!("expected cookieless loopback request to be allowed"),
        }
    }

    #[test]
    fn other_loopback_paths_are_allowed() {
        let peer = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
        match GateImpl.handle(&loopback_request("/"), peer) {
            GateVerdict::Allow => {}
            _ => panic!("expected loopback request to be allowed"),
        }
    }
}
