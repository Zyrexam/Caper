//! The dev server binds 0.0.0.0, so the peer's address is the trust boundary:
//! loopback is served, everything else is disconnected without a reply.

use std::net::IpAddr;

use rustex1_devserver::{Gate, GateReply, GateVerdict, Request};

pub struct GateImpl;

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
        if req.path == "/login" {
            // No login route exists; reject instead of serving the SPA fallback.
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
    fn other_loopback_paths_are_allowed() {
        let peer = Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
        match GateImpl.handle(&loopback_request("/"), peer) {
            GateVerdict::Allow => {}
            _ => panic!("expected loopback request to be allowed"),
        }
    }
}
