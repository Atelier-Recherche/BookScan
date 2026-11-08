//! Pick a usable LAN IPv4 for phone ↔ PC Wi‑Fi (skip WSL / APIPA / virtual).

use std::net::Ipv4Addr;

pub fn preferred_lan_ipv4() -> String {
    if let Ok(adapters) = local_ip_address::list_afinet_netifas() {
        let mut candidates: Vec<(u8, String)> = Vec::new();
        for (name, ip) in adapters {
            let std::net::IpAddr::V4(v4) = ip else { continue };
            if let Some(score) = score_iface(&name, v4) {
                candidates.push((score, v4.to_string()));
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        if let Some((_, ip)) = candidates.first() {
            return ip.clone();
        }
    }
    local_ip_address::local_ip()
        .ok()
        .and_then(|ip| match ip {
            std::net::IpAddr::V4(v4) => Some(v4.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "127.0.0.1".into())
}

fn score_iface(name: &str, ip: Ipv4Addr) -> Option<u8> {
    if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() {
        return None;
    }
    let n = name.to_ascii_lowercase();
    // Skip typical virtual adapters
    if n.contains("wsl")
        || n.contains("vethernet")
        || n.contains("hyper-v")
        || n.contains("virtualbox")
        || n.contains("vmware")
        || n.contains("docker")
        || n.contains("bluetooth")
        || n.contains("loopback")
    {
        return None;
    }
    // Prefer RFC1918 home LAN
    let octets = ip.octets();
    let is_home = octets[0] == 192 && octets[1] == 168
        || octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]));
    if !is_home {
        return None;
    }
    let mut score = 50u8;
    if n.contains("wi-fi") || n.contains("wifi") || n.contains("wlan") {
        score += 40;
    }
    if n.contains("ethernet") || n.contains("eth") || n.contains("lan") {
        score += 35;
    }
    if octets[0] == 192 && octets[1] == 168 {
        score += 10;
    }
    Some(score)
}
