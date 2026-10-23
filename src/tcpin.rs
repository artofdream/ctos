//! Inbound TCP: one passive open (listen/accept) over QEMU user-net
//! `hostfwd` (Track N / ADR-108). Builds on the thin active-open TCP of
//! ADR-074 and the same virtio-net path; the kernel still owns the NIC.
//!
//! One listening port, one connection, then done. The host connects through
//! `hostfwd=tcp:127.0.0.1:<p>-:8080`; SLIRP turns that into a SYN from
//! 10.0.2.2 to 10.0.2.15:8080. The guest answers SYN-ACK, takes the ACK,
//! reads one line, sends `ctos-in:<line>`, then closes (FIN), and ACKs the
//! host's FIN. A SYN to any other port (the negative, guest port 8081) gets
//! an RST. Segments with a bad TCP checksum are dropped and counted.
//!
//! Not a sockets API. No socket table, no backlog, no retransmit timer, no
//! window management beyond a fixed advertised window, no options (MSS is
//! not sent), no IPv6, no EL0 SVC. Not a TCP product.

use core::fmt::Write;

use crate::uart;
use crate::virtio::{self, NET_FRAME_MAX};

pub const LISTEN_PORT: u16 = 8080;
/// hostfwd also maps a second port to this guest port, where nothing listens.
pub const CLOSED_PORT: u16 = 8081;
const GUEST_IP: [u8; 4] = [10, 0, 2, 15];
const ISS: u32 = 0x3000_0000;
const REPLY_PREFIX: &[u8] = b"ctos-in:";
const LINE_MAX: usize = 64;
const DEADLINE_MS: u64 = 15_000;

const FIN: u8 = 0x01;
const SYN: u8 = 0x02;
const RST: u8 = 0x04;
const PSH: u8 = 0x08;
const ACK: u8 = 0x10;

/// A parsed inbound IPv4/TCP segment addressed to the guest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seg {
    pub src_mac: [u8; 6],
    pub src_ip: [u8; 4],
    pub sport: u16,
    pub dport: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    pub off: usize,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parse {
    NotTcp,
    BadChecksum,
    Tcp(Seg),
}

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// TCP checksum over the IPv4 pseudo-header + segment, in place-safe form.
pub fn tcp_csum(src: [u8; 4], dst: [u8; 4], tcp: &[u8]) -> u16 {
    let mut buf = [0u8; 12 + NET_FRAME_MAX];
    if tcp.len() > NET_FRAME_MAX {
        return 0;
    }
    buf[0..4].copy_from_slice(&src);
    buf[4..8].copy_from_slice(&dst);
    buf[9] = 6;
    buf[10..12].copy_from_slice(&(tcp.len() as u16).to_be_bytes());
    buf[12..12 + tcp.len()].copy_from_slice(tcp);
    virtio::inet_checksum(&buf[..12 + tcp.len()])
}

/// Parse a frame; only IPv4/TCP to 10.0.2.15 with a valid checksum is `Tcp`.
pub fn parse(frame: &[u8], len: usize) -> Parse {
    if len < 14 + 20 + 20 || len > frame.len() || frame[12] != 0x08 || frame[13] != 0x00 {
        return Parse::NotTcp;
    }
    let ihl = (frame[14] & 0x0f) as usize * 4;
    if frame[14] >> 4 != 4 || ihl < 20 || frame[23] != 6 {
        return Parse::NotTcp;
    }
    let ip_total = be16(frame, 16) as usize;
    if ip_total < ihl + 20 || 14 + ip_total > len {
        return Parse::NotTcp;
    }
    if frame[30..34] != GUEST_IP {
        return Parse::NotTcp;
    }
    // Fragments are not handled: MF set or a non-zero offset is dropped.
    if be16(frame, 20) & 0x3fff != 0 {
        return Parse::NotTcp;
    }
    let t = 14 + ihl;
    let tlen = ip_total - ihl;
    let doff = (frame[t + 12] >> 4) as usize * 4;
    if doff < 20 || doff > tlen {
        return Parse::NotTcp;
    }
    let mut src_ip = [0u8; 4];
    src_ip.copy_from_slice(&frame[26..30]);
    if tcp_csum(src_ip, GUEST_IP, &frame[t..t + tlen]) != 0 {
        return Parse::BadChecksum;
    }
    let mut src_mac = [0u8; 6];
    src_mac.copy_from_slice(&frame[6..12]);
    Parse::Tcp(Seg {
        src_mac,
        src_ip,
        sport: be16(frame, t),
        dport: be16(frame, t + 2),
        seq: be32(frame, t + 4),
        ack: be32(frame, t + 8),
        flags: frame[t + 13],
        off: t + doff,
        len: tlen - doff,
    })
}

/// Build a reply segment to `to` (swap ports/addresses). Returns frame length.
pub fn build(
    out: &mut [u8; NET_FRAME_MAX],
    mac: &[u8; 6],
    to: &Seg,
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) -> usize {
    let tlen = 20 + payload.len();
    let ip_total = 20 + tlen;
    if 14 + ip_total > NET_FRAME_MAX {
        return 0;
    }
    out.fill(0);
    out[0..6].copy_from_slice(&to.src_mac);
    out[6..12].copy_from_slice(mac);
    out[12] = 0x08;
    out[14] = 0x45;
    out[16..18].copy_from_slice(&(ip_total as u16).to_be_bytes());
    out[19] = 0x08; // id
    out[20] = 0x40; // DF
    out[22] = 64;
    out[23] = 6;
    out[26..30].copy_from_slice(&GUEST_IP);
    out[30..34].copy_from_slice(&to.src_ip);
    let c = virtio::inet_checksum(&out[14..34]);
    out[24..26].copy_from_slice(&c.to_be_bytes());
    let t = 34;
    out[t..t + 2].copy_from_slice(&to.dport.to_be_bytes());
    out[t + 2..t + 4].copy_from_slice(&to.sport.to_be_bytes());
    out[t + 4..t + 8].copy_from_slice(&seq.to_be_bytes());
    out[t + 8..t + 12].copy_from_slice(&ack.to_be_bytes());
    out[t + 12] = 0x50;
    out[t + 13] = flags;
    out[t + 14..t + 16].copy_from_slice(&0x2000u16.to_be_bytes());
    out[t + 20..t + 20 + payload.len()].copy_from_slice(payload);
    let c = tcp_csum(GUEST_IP, to.src_ip, &out[t..t + tlen]);
    out[t + 16..t + 18].copy_from_slice(&c.to_be_bytes());
    14 + ip_total
}

/// RFC 793 reset for a segment to a closed port. `None` if `s` is an RST.
pub fn rst_for(s: &Seg) -> Option<(u32, u32, u8)> {
    if s.flags & RST != 0 {
        return None;
    }
    if s.flags & ACK != 0 {
        Some((s.ack, 0, RST))
    } else {
        let mut seglen = s.len as u32;
        if s.flags & SYN != 0 {
            seglen += 1;
        }
        if s.flags & FIN != 0 {
            seglen += 1;
        }
        Some((0, s.seq.wrapping_add(seglen), RST | ACK))
    }
}

/// Answer `who-has 10.0.2.15` so SLIRP can reach us. Returns frame length.
fn arp_reply(frame: &[u8], len: usize, mac: &[u8; 6], out: &mut [u8; NET_FRAME_MAX]) -> usize {
    if len < 42 || frame[12] != 0x08 || frame[13] != 0x06 || be16(frame, 20) != 1 {
        return 0;
    }
    if frame[38..42] != GUEST_IP {
        return 0;
    }
    out.fill(0);
    out[0..6].copy_from_slice(&frame[22..28]);
    out[6..12].copy_from_slice(mac);
    out[12] = 0x08;
    out[13] = 0x06;
    out[14..20].copy_from_slice(&frame[14..20]);
    out[21] = 2;
    out[22..28].copy_from_slice(mac);
    out[28..32].copy_from_slice(&GUEST_IP);
    out[32..38].copy_from_slice(&frame[22..28]);
    out[38..42].copy_from_slice(&frame[28..32]);
    42
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum St {
    Listen,
    SynRcvd,
    Established,
    FinWait,
    Done,
}

fn tx(out: &[u8], n: usize) -> bool {
    n != 0 && virtio::net_raw_tx(&out[..n])
}

/// Serial proof (ADR-108). Prints `net: tcp-in-listen port=8080` as the cue
/// the host waits for, then needs both the RST (closed port) and the full
/// accept → data → close exchange before `net: tcp-in-ok`.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    let Some(mac) = virtio::guest_mac() else {
        uart::write_str_raw("net: tcp-in-fail reason=no-nic\n");
        return false;
    };
    {
        let mut w = uart::raw();
        let _ = writeln!(w, "net: tcp-in-listen port={} closed={}", LISTEN_PORT, CLOSED_PORT);
    }
    let start = crate::timer::cntpct();
    let mut freq: u64;
    unsafe {
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) freq);
    }
    if freq == 0 {
        freq = 62_500_000;
    }
    let deadline = start.saturating_add(freq.saturating_mul(DEADLINE_MS) / 1000);

    let mut st = St::Listen;
    let mut peer: Option<Seg> = None;
    let mut rcv_nxt = 0u32;
    let mut snd_nxt = ISS;
    let mut line = [0u8; LINE_MAX];
    let mut line_len = 0usize;
    let mut rst_sent = false;
    let mut bad_csum = 0u32;
    let mut stray_rst = 0u32;
    let mut frame = [0u8; NET_FRAME_MAX];
    let mut out = [0u8; NET_FRAME_MAX];
    let mut fin_acked = false;
    let mut peer_fin = false;
    let mut w = uart::raw();

    while crate::timer::cntpct() < deadline && !(st == St::Done && rst_sent) {
        let Some(n) = virtio::net_raw_rx(&mut frame, 200) else {
            continue;
        };
        let a = arp_reply(&frame, n, &mac, &mut out);
        if a != 0 {
            let _ = tx(&out, a);
            continue;
        }
        let s = match parse(&frame, n) {
            Parse::NotTcp => continue,
            Parse::BadChecksum => {
                bad_csum += 1;
                continue;
            }
            Parse::Tcp(s) => s,
        };
        if s.dport != LISTEN_PORT {
            if let Some((seq, ack, fl)) = rst_for(&s) {
                let m = build(&mut out, &mac, &s, seq, ack, fl, &[]);
                let sent = tx(&out, m);
                // Only the hostfwd'd closed port is the negative; late
                // segments of earlier outbound probes also get an RST but
                // do not count.
                if sent && s.dport == CLOSED_PORT && s.flags & SYN != 0 && !rst_sent {
                    rst_sent = true;
                    let _ = writeln!(w, "net: tcp-in-rst port={}", s.dport);
                } else if sent {
                    stray_rst += 1;
                }
            }
            continue;
        }
        // Only the first accepted peer; other 4-tuples to 8080 get RST.
        if let Some(p) = peer {
            if p.sport != s.sport || p.src_ip != s.src_ip {
                if let Some((seq, ack, fl)) = rst_for(&s) {
                    let m = build(&mut out, &mac, &s, seq, ack, fl, &[]);
                    let _ = tx(&out, m);
                }
                continue;
            }
        }
        if s.flags & RST != 0 {
            let _ = writeln!(w, "net: tcp-in-fail reason=peer-rst state={:?}", st);
            return false;
        }
        match st {
            St::Listen => {
                if s.flags & SYN == 0 || s.flags & ACK != 0 {
                    continue;
                }
                peer = Some(s);
                rcv_nxt = s.seq.wrapping_add(1);
                let m = build(&mut out, &mac, &s, ISS, rcv_nxt, SYN | ACK, &[]);
                if !tx(&out, m) {
                    continue;
                }
                snd_nxt = ISS.wrapping_add(1);
                st = St::SynRcvd;
                let _ = writeln!(w, "net: tcp-in-syn sport={}", s.sport);
            }
            St::SynRcvd | St::Established => {
                if st == St::SynRcvd {
                    if s.flags & SYN != 0 && s.flags & ACK == 0 {
                        // SLIRP retransmitted its SYN: resend SYN-ACK.
                        let m = build(&mut out, &mac, &s, ISS, rcv_nxt, SYN | ACK, &[]);
                        let _ = tx(&out, m);
                        continue;
                    }
                    if s.flags & ACK == 0 || s.ack != snd_nxt {
                        continue;
                    }
                    st = St::Established;
                    let _ = writeln!(w, "net: tcp-in-est");
                }
                if s.len == 0 {
                    continue;
                }
                if s.seq != rcv_nxt {
                    // Out of order or duplicate: re-ACK what we have.
                    let m = build(&mut out, &mac, &s, snd_nxt, rcv_nxt, ACK, &[]);
                    let _ = tx(&out, m);
                    continue;
                }
                let take = s.len.min(LINE_MAX - line_len);
                line[line_len..line_len + take].copy_from_slice(&frame[s.off..s.off + take]);
                line_len += take;
                rcv_nxt = rcv_nxt.wrapping_add(s.len as u32);
                let have_nl = line[..line_len].contains(&b'\n');
                if !have_nl && line_len < LINE_MAX {
                    let m = build(&mut out, &mac, &s, snd_nxt, rcv_nxt, ACK, &[]);
                    let _ = tx(&out, m);
                    continue;
                }
                let _ = writeln!(w, "net: tcp-in-rx bytes={}", line_len);
                let mut reply = [0u8; LINE_MAX + 8];
                reply[..REPLY_PREFIX.len()].copy_from_slice(REPLY_PREFIX);
                reply[REPLY_PREFIX.len()..REPLY_PREFIX.len() + line_len]
                    .copy_from_slice(&line[..line_len]);
                let rl = REPLY_PREFIX.len() + line_len;
                let m = build(&mut out, &mac, &s, snd_nxt, rcv_nxt, PSH | ACK, &reply[..rl]);
                if !tx(&out, m) {
                    let _ = writeln!(w, "net: tcp-in-fail reason=tx");
                    return false;
                }
                snd_nxt = snd_nxt.wrapping_add(rl as u32);
                let _ = writeln!(w, "net: tcp-in-tx bytes={}", rl);
                // Active close from the guest side.
                let m = build(&mut out, &mac, &s, snd_nxt, rcv_nxt, FIN | ACK, &[]);
                if !tx(&out, m) {
                    let _ = writeln!(w, "net: tcp-in-fail reason=tx-fin");
                    return false;
                }
                snd_nxt = snd_nxt.wrapping_add(1);
                st = St::FinWait;
            }
            St::FinWait => {
                if s.flags & ACK != 0 && s.ack == snd_nxt {
                    fin_acked = true;
                }
                if s.flags & FIN != 0 && s.seq.wrapping_add(s.len as u32) == rcv_nxt {
                    rcv_nxt = rcv_nxt.wrapping_add(1);
                    peer_fin = true;
                    let m = build(&mut out, &mac, &s, snd_nxt, rcv_nxt, ACK, &[]);
                    let _ = tx(&out, m);
                }
                if fin_acked && peer_fin {
                    st = St::Done;
                    let _ = writeln!(w, "net: tcp-in-fin closed");
                }
            }
            St::Done => {}
        }
    }
    if st != St::Done || !rst_sent {
        let _ = writeln!(
            w,
            "net: tcp-in-fail reason=timeout state={:?} rst={} bad-csum={} stray-rst={}",
            st, rst_sent, bad_csum, stray_rst
        );
        return false;
    }
    let _ = writeln!(w, "net: tcp-in-ok bad-csum={} stray-rst={}", bad_csum, stray_rst);
    true
}

#[cfg(test)]
fn sample_syn(dport: u16) -> ([u8; NET_FRAME_MAX], usize) {
    // Build a SYN "from" 10.0.2.2:40000 by reflecting through `build`.
    let peer = Seg {
        src_mac: [0x52, 0x54, 0, 0x12, 0x34, 0x56],
        src_ip: GUEST_IP,
        sport: dport,
        dport: 40000,
        seq: 0,
        ack: 0,
        flags: 0,
        off: 0,
        len: 0,
    };
    let mut f = [0u8; NET_FRAME_MAX];
    let n = build(&mut f, &[0x52, 0x55, 10, 0, 2, 2], &peer, 0x1234, 0, SYN, &[]);
    // `build` writes src = GUEST_IP; rewrite to 10.0.2.2 and fix checksums.
    f[26..30].copy_from_slice(&[10, 0, 2, 2]);
    f[24] = 0;
    f[25] = 0;
    let c = virtio::inet_checksum(&f[14..34]);
    f[24..26].copy_from_slice(&c.to_be_bytes());
    f[50] = 0;
    f[51] = 0;
    let c = tcp_csum([10, 0, 2, 2], GUEST_IP, &f[34..n]);
    f[50..52].copy_from_slice(&c.to_be_bytes());
    (f, n)
}

#[cfg(test)]
#[test_case]
fn tcpin_parse_syn_and_reject_bad_checksum() {
    let (mut f, n) = sample_syn(LISTEN_PORT);
    match parse(&f, n) {
        Parse::Tcp(s) => {
            assert_eq!(s.sport, 40000);
            assert_eq!(s.dport, LISTEN_PORT);
            assert_eq!(s.seq, 0x1234);
            assert_eq!(s.flags, SYN);
            assert_eq!(s.src_ip, [10, 0, 2, 2]);
        }
        other => panic!("SYN must parse, got {:?}", other),
    }
    f[40] ^= 0xFF; // corrupt the sequence number
    assert_eq!(parse(&f, n), Parse::BadChecksum);
}

#[cfg(test)]
#[test_case]
fn tcpin_rst_for_closed_port_follows_rfc793() {
    let (f, n) = sample_syn(CLOSED_PORT);
    let Parse::Tcp(s) = parse(&f, n) else { panic!("SYN must parse") };
    assert_eq!(rst_for(&s), Some((0, 0x1235, RST | ACK)));
    let mut a = s;
    a.flags = ACK;
    a.ack = 77;
    assert_eq!(rst_for(&a), Some((77, 0, RST)));
    a.flags = RST;
    assert_eq!(rst_for(&a), None, "never answer an RST");
}

#[cfg(test)]
#[test_case]
fn tcpin_reply_checksum_valid_and_addresses_swapped() {
    let (f, n) = sample_syn(LISTEN_PORT);
    let Parse::Tcp(s) = parse(&f, n) else { panic!() };
    let mut out = [0u8; NET_FRAME_MAX];
    let m = build(&mut out, &[2, 0, 0, 0, 0, 1], &s, ISS, 0x1235, SYN | ACK, b"x");
    assert!(m > 0);
    assert_eq!(&out[30..34], &[10, 0, 2, 2]);
    assert_eq!(virtio::inet_checksum(&out[14..34]), 0);
    assert_eq!(tcp_csum(GUEST_IP, [10, 0, 2, 2], &out[34..m]), 0);
    assert_eq!(be16(&out, 34), LISTEN_PORT);
    assert_eq!(be16(&out, 36), 40000);
}

#[cfg(test)]
#[test_case]
fn tcpin_drops_non_tcp_and_fragments() {
    let (mut f, n) = sample_syn(LISTEN_PORT);
    f[20] = 0x20; // MF
    assert_eq!(parse(&f, n), Parse::NotTcp);
    let (mut g, n) = sample_syn(LISTEN_PORT);
    g[33] = 99; // not our address
    assert_eq!(parse(&g, n), Parse::NotTcp);
    assert_eq!(parse(&g, 20), Parse::NotTcp);
}
