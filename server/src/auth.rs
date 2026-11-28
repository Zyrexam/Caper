//! The dev server binds 0.0.0.0, so the peer's address is the trust boundary:
//! loopback is served, everything else is disconnected without a reply.

use std::net::IpAddr;

use rustex1_devserver::{Gate, GateVerdict, Request};

pub struct GateImpl;

fn is_loopback(peer: Option<IpAddr>) -> bool {
    peer.map(|p| p.is_loopback()).unwrap_or(false)
}

impl Gate for GateImpl {
    fn early(&self, _req: &Request, peer: Option<IpAddr>) -> bool {
        is_loopback(peer)
    }

    fn handle(&self, req: &Request, peer: Option<IpAddr>) -> GateVerdict {
        if self.early(req, peer) {
            GateVerdict::Allow
        } else {
            GateVerdict::HangUp
        }
    }
}
