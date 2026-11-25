use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use renet::{
    Bytes, ChannelConfig, ClientId, ConnectionConfig, RenetClient,
    RenetServer, SendType, ServerEvent,
};

pub const CHANNEL_UNRELIABLE: u8 = 0;
pub const CHANNEL_RELIABLE_ORDERED: u8 = 1;
pub const CHANNEL_RELIABLE_UNORDERED: u8 = 2;

const MAGIC_CONNECT_REQ: &[u8] = b"MD_CONNECT_REQ";
const MAGIC_CONNECT_ACK: &[u8] = b"MD_CONNECT_ACK:";
const MAGIC_DISCONNECT: &[u8] = b"MD_DISCONNECT";
const RPC_MAGIC_PREFIX: &[u8] = b"MDRPC:";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RpcMessage {
    pub fn_name: String,
    pub args_json: String,
}

pub fn encode_rpc(fn_name: &str, args_json: &str) -> Vec<u8> {
    let msg = RpcMessage {
        fn_name: fn_name.to_string(),
        args_json: args_json.to_string(),
    };
    let mut out = RPC_MAGIC_PREFIX.to_vec();
    if let Ok(json_bytes) = serde_json::to_vec(&msg) {
        out.extend_from_slice(&json_bytes);
    }
    out
}

pub fn decode_rpc(data: &[u8]) -> Option<RpcMessage> {
    let payload = data.strip_prefix(RPC_MAGIC_PREFIX)?;
    serde_json::from_slice::<RpcMessage>(payload).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkRole {
    Disconnected,
    Server,
    Client,
}

pub fn create_default_connection_config() -> ConnectionConfig {
    let client_channels = vec![
        // Channel 0: Unreliable (Input & high-frequency transform sync)
        ChannelConfig {
            channel_id: CHANNEL_UNRELIABLE,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::Unreliable,
        },
        // Channel 1: ReliableOrdered (Spawns, triggers, critical gameplay events)
        ChannelConfig {
            channel_id: CHANNEL_RELIABLE_ORDERED,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(150),
            },
        },
        // Channel 2: ReliableUnordered (Discreet confirmations & heartbeats)
        ChannelConfig {
            channel_id: CHANNEL_RELIABLE_UNORDERED,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::ReliableUnordered {
                resend_time: Duration::from_millis(150),
            },
        },
    ];

    let server_channels = client_channels.clone();

    ConnectionConfig {
        available_bytes_per_tick: 64_000,
        server_channels_config: server_channels,
        client_channels_config: client_channels,
    }
}

pub struct NetworkManager {
    pub role: NetworkRole,
    pub server: Option<RenetServer>,
    pub client: Option<RenetClient>,
    pub socket: Option<UdpSocket>,
    pub client_addrs: HashMap<ClientId, SocketAddr>,
    pub addr_to_client: HashMap<SocketAddr, ClientId>,
    pub next_client_id: u64,
    pub server_addr: Option<SocketAddr>,
    pub local_client_id: u64,
    pub last_update: Instant,
    pub connection_events: Vec<ServerEvent>,
    pub connect_request_timer: Instant,
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            role: NetworkRole::Disconnected,
            server: None,
            client: None,
            socket: None,
            client_addrs: HashMap::new(),
            addr_to_client: HashMap::new(),
            next_client_id: 1,
            server_addr: None,
            local_client_id: 0,
            last_update: Instant::now(),
            connection_events: Vec::new(),
            connect_request_timer: Instant::now(),
        }
    }

    pub fn host(port: u16) -> Result<Self, String> {
        let addr: SocketAddr = format!("0.0.0.0:{}", port)
            .parse()
            .map_err(|e| format!("Invalid host address: {}", e))?;
        let socket = UdpSocket::bind(addr)
            .map_err(|e| format!("Failed to bind UDP server on port {}: {}", port, e))?;
        socket.set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking UDP: {}", e))?;

        let config = create_default_connection_config();
        let server = RenetServer::new(config);

        Ok(Self {
            role: NetworkRole::Server,
            server: Some(server),
            client: None,
            socket: Some(socket),
            client_addrs: HashMap::new(),
            addr_to_client: HashMap::new(),
            next_client_id: 1,
            server_addr: None,
            local_client_id: 0,
            last_update: Instant::now(),
            connection_events: Vec::new(),
            connect_request_timer: Instant::now(),
        })
    }

    pub fn connect(server_addr_str: &str) -> Result<Self, String> {
        let server_addr: SocketAddr = server_addr_str
            .parse()
            .map_err(|e| format!("Invalid server address '{}': {}", server_addr_str, e))?;
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Failed to bind client UDP socket: {}", e))?;
        socket.set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking UDP: {}", e))?;

        // Send initial connect request packet
        let _ = socket.send_to(MAGIC_CONNECT_REQ, server_addr);

        let config = create_default_connection_config();
        let client = RenetClient::new(config);

        Ok(Self {
            role: NetworkRole::Client,
            server: None,
            client: Some(client),
            socket: Some(socket),
            client_addrs: HashMap::new(),
            addr_to_client: HashMap::new(),
            next_client_id: 1,
            server_addr: Some(server_addr),
            local_client_id: 0,
            last_update: Instant::now(),
            connection_events: Vec::new(),
            connect_request_timer: Instant::now(),
        })
    }

    pub fn disconnect(&mut self) {
        if let Some(socket) = &self.socket {
            if self.role == NetworkRole::Client {
                if let Some(srv) = self.server_addr {
                    let _ = socket.send_to(MAGIC_DISCONNECT, srv);
                }
            } else if self.role == NetworkRole::Server {
                for addr in self.client_addrs.values() {
                    let _ = socket.send_to(MAGIC_DISCONNECT, addr);
                }
            }
        }
        self.role = NetworkRole::Disconnected;
        self.server = None;
        self.client = None;
        self.socket = None;
        self.client_addrs.clear();
        self.addr_to_client.clear();
        self.server_addr = None;
        self.local_client_id = 0;
    }

    pub fn update(&mut self, dt: Duration) {
        self.last_update = Instant::now();

        if let Some(socket) = &self.socket {
            let mut buf = [0u8; 4096];

            match self.role {
                NetworkRole::Server => {
                    let mut new_clients = Vec::new();
                    let mut disc_clients = Vec::new();

                    // 1. Drain incoming UDP datagrams
                    while let Ok((len, from_addr)) = socket.recv_from(&mut buf) {
                        let packet = &buf[..len];
                        if packet == MAGIC_CONNECT_REQ {
                            if !self.addr_to_client.contains_key(&from_addr) {
                                let new_id = self.next_client_id;
                                self.next_client_id += 1;
                                new_clients.push((new_id, from_addr));
                            }
                            continue;
                        }
                        if packet == MAGIC_DISCONNECT {
                            if let Some(&cid) = self.addr_to_client.get(&from_addr) {
                                disc_clients.push(cid);
                            }
                            continue;
                        }

                        if let Some(&cid) = self.addr_to_client.get(&from_addr) {
                            if let Some(server) = &mut self.server {
                                let _ = server.process_packet_from(packet, cid);
                            }
                        }
                    }

                    // Register new connected clients
                    for (cid, addr) in new_clients {
                        self.client_addrs.insert(cid, addr);
                        self.addr_to_client.insert(addr, cid);
                        if let Some(server) = &mut self.server {
                            server.add_connection(cid);
                        }
                        // Send connection ACK with assigned client ID
                        let ack_msg = format!("MD_CONNECT_ACK:{}", cid);
                        let _ = socket.send_to(ack_msg.as_bytes(), addr);
                        self.connection_events.push(ServerEvent::ClientConnected { client_id: cid });
                    }

                    // Process disconnects
                    for cid in disc_clients {
                        if let Some(server) = &mut self.server {
                            server.remove_connection(cid);
                        }
                        if let Some(addr) = self.client_addrs.remove(&cid) {
                            self.addr_to_client.remove(&addr);
                        }
                        self.connection_events.push(ServerEvent::ClientDisconnected {
                            client_id: cid,
                            reason: renet::DisconnectReason::DisconnectedByClient,
                        });
                    }

                    // 2. Update Renet server clock
                    if let Some(server) = &mut self.server {
                        server.update(dt);

                        while let Some(event) = server.get_event() {
                            self.connection_events.push(event);
                        }

                        // 3. Send out generated packets to each client
                        for (&cid, &addr) in &self.client_addrs {
                            if let Ok(packets) = server.get_packets_to_send(cid) {
                                for payload in packets {
                                    let _ = socket.send_to(&payload, addr);
                                }
                            }
                        }
                    }
                }
                NetworkRole::Client => {
                    let mut handshake_ack_id = None;

                    // 1. Drain incoming UDP datagrams
                    while let Ok((len, from_addr)) = socket.recv_from(&mut buf) {
                        let packet = &buf[..len];
                        if packet.starts_with(MAGIC_CONNECT_ACK) {
                            if let Ok(id_str) = std::str::from_utf8(&packet[MAGIC_CONNECT_ACK.len()..]) {
                                if let Ok(cid) = id_str.parse::<u64>() {
                                    handshake_ack_id = Some(cid);
                                }
                            }
                            continue;
                        }
                        if packet == MAGIC_DISCONNECT {
                            if let Some(client) = &mut self.client {
                                client.disconnect();
                            }
                            continue;
                        }

                        if Some(from_addr) == self.server_addr {
                            if let Some(client) = &mut self.client {
                                client.process_packet(packet);
                            }
                        }
                    }

                    if let Some(cid) = handshake_ack_id {
                        self.local_client_id = cid;
                        if let Some(client) = &mut self.client {
                            client.set_connected();
                        }
                    }

                    // Periodic connection request retries if still connecting
                    if let Some(client) = &self.client {
                        if !client.is_connected() && self.connect_request_timer.elapsed() > Duration::from_millis(300) {
                            self.connect_request_timer = Instant::now();
                            if let Some(srv) = self.server_addr {
                                let _ = socket.send_to(MAGIC_CONNECT_REQ, srv);
                            }
                        }
                    }

                    // 2. Update Renet client clock
                    if let Some(client) = &mut self.client {
                        client.update(dt);

                        // 3. Send generated packets to server
                        if let Some(srv) = self.server_addr {
                            let packets = client.get_packets_to_send();
                            for payload in packets {
                                let _ = socket.send_to(&payload, srv);
                            }
                        }
                    }
                }
                NetworkRole::Disconnected => {}
            }
        }
    }

    pub fn send_message(&mut self, client_id: ClientId, channel_id: u8, message: &[u8]) {
        if let Some(server) = &mut self.server {
            server.send_message(client_id, channel_id, Bytes::copy_from_slice(message));
        }
    }

    pub fn broadcast_message(&mut self, channel_id: u8, message: &[u8]) {
        if let Some(server) = &mut self.server {
            server.broadcast_message(channel_id, Bytes::copy_from_slice(message));
        }
    }

    pub fn send(&mut self, channel_id: u8, message: &[u8]) {
        if let Some(client) = &mut self.client {
            client.send_message(channel_id, Bytes::copy_from_slice(message));
        }
    }

    pub fn receive_messages(&mut self) -> Vec<(Option<ClientId>, u8, Vec<u8>)> {
        let mut messages = Vec::new();

        if let Some(server) = &mut self.server {
            let client_ids = server.clients_id();
            for cid in client_ids {
                for ch in [CHANNEL_UNRELIABLE, CHANNEL_RELIABLE_ORDERED, CHANNEL_RELIABLE_UNORDERED] {
                    while let Some(msg) = server.receive_message(cid, ch) {
                        messages.push((Some(cid), ch, msg.to_vec()));
                    }
                }
            }
        } else if let Some(client) = &mut self.client {
            for ch in [CHANNEL_UNRELIABLE, CHANNEL_RELIABLE_ORDERED, CHANNEL_RELIABLE_UNORDERED] {
                while let Some(msg) = client.receive_message(ch) {
                    messages.push((None, ch, msg.to_vec()));
                }
            }
        }

        messages
    }

    pub fn is_server(&self) -> bool {
        self.role == NetworkRole::Server
    }

    pub fn is_client(&self) -> bool {
        self.role == NetworkRole::Client
    }

    pub fn is_connected(&self) -> bool {
        match self.role {
            NetworkRole::Server => true,
            NetworkRole::Client => self.client.as_ref().map(|c| c.is_connected()).unwrap_or(false),
            NetworkRole::Disconnected => false,
        }
    }

    pub fn peer_count(&self) -> usize {
        match self.role {
            NetworkRole::Server => self.server.as_ref().map(|s| s.connected_clients()).unwrap_or(0),
            NetworkRole::Client => if self.is_connected() { 1 } else { 0 },
            NetworkRole::Disconnected => 0,
        }
    }

    pub fn rtt(&self, client_id: Option<ClientId>) -> f64 {
        match self.role {
            NetworkRole::Server => {
                if let Some(cid) = client_id {
                    self.server.as_ref().map(|s| s.rtt(cid)).unwrap_or(0.0)
                } else if let Some(&first_id) = self.client_addrs.keys().next() {
                    self.server.as_ref().map(|s| s.rtt(first_id)).unwrap_or(0.0)
                } else {
                    0.0
                }
            }
            NetworkRole::Client => self.client.as_ref().map(|c| c.rtt()).unwrap_or(0.0),
            NetworkRole::Disconnected => 0.0,
        }
    }

    pub fn tx_rx_stats(&self) -> (f64, f64) {
        match self.role {
            NetworkRole::Server => {
                let mut tx = 0.0;
                let mut rx = 0.0;
                if let Some(server) = &self.server {
                    for &cid in self.client_addrs.keys() {
                        tx += server.bytes_sent_per_sec(cid);
                        rx += server.bytes_received_per_sec(cid);
                    }
                }
                (tx, rx)
            }
            NetworkRole::Client => {
                if let Some(client) = &self.client {
                    (client.bytes_sent_per_sec(), client.bytes_received_per_sec())
                } else {
                    (0.0, 0.0)
                }
            }
            NetworkRole::Disconnected => (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EntityTransformSnapshot {
    pub net_id: u64,
    pub pos: [f32; 3],
    pub rot: [f32; 4], // Quat [x, y, z, w]
    pub scale: [f32; 3],
    pub anim_clip: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WorldSnapshotMessage {
    pub timestamp: f64,
    pub entities: Vec<EntityTransformSnapshot>,
}

impl WorldSnapshotMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_snapshot_serialization_roundtrip() {
        let snap = WorldSnapshotMessage {
            timestamp: 123.456,
            entities: vec![EntityTransformSnapshot {
                net_id: 101,
                pos: [1.0, 2.0, 3.0],
                rot: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
                anim_clip: Some("run".to_string()),
            }],
        };
        let bytes = snap.to_bytes().expect("Serialize snapshot failed");
        assert!(!bytes.is_empty());
        let decoded = WorldSnapshotMessage::from_bytes(&bytes).expect("Deserialize snapshot failed");
        assert_eq!(decoded, snap);
    }

    #[test]
    fn test_network_connection_config_channels() {
        let config = create_default_connection_config();
        assert_eq!(config.client_channels_config.len(), 3);
        assert_eq!(config.server_channels_config.len(), 3);

        assert_eq!(config.client_channels_config[0].channel_id, 0);
        assert!(matches!(config.client_channels_config[0].send_type, SendType::Unreliable));

        assert_eq!(config.client_channels_config[1].channel_id, 1);
        assert!(matches!(config.client_channels_config[1].send_type, SendType::ReliableOrdered { .. }));

        assert_eq!(config.client_channels_config[2].channel_id, 2);
        assert!(matches!(config.client_channels_config[2].send_type, SendType::ReliableUnordered { .. }));
    }

    #[test]
    fn test_net_udp_loopback_3_channels() {
        let mut server = NetworkManager::host(0).expect("Host server failed");
        let server_port = server.socket.as_ref().unwrap().local_addr().unwrap().port();

        let mut client = NetworkManager::connect(&format!("127.0.0.1:{}", server_port))
            .expect("Connect client failed");

        let dt = Duration::from_millis(16);

        // Run ticks for handshake exchange
        for _ in 0..10 {
            server.update(dt);
            client.update(dt);
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(client.is_connected(), "Client should be connected after handshake");
        assert_eq!(server.peer_count(), 1, "Server should register 1 connected client");

        let client_id = server.client_addrs.keys().copied().next().unwrap();

        // 1. Send on Channel 0 (Unreliable)
        client.send(CHANNEL_UNRELIABLE, b"unreliable_pos_delta");

        // 2. Send on Channel 1 (ReliableOrdered)
        client.send(CHANNEL_RELIABLE_ORDERED, b"player_spawn_event");

        // 3. Send on Channel 2 (ReliableUnordered)
        client.send(CHANNEL_RELIABLE_UNORDERED, b"heartbeat_ping");

        // Server updates to receive messages
        for _ in 0..5 {
            client.update(dt);
            server.update(dt);
            std::thread::sleep(Duration::from_millis(5));
        }

        let srv_msgs = server.receive_messages();
        assert!(!srv_msgs.is_empty(), "Server should receive client messages");

        let mut has_unreliable = false;
        let mut has_ordered = false;
        let mut has_unordered = false;

        for (from, ch, data) in srv_msgs {
            assert_eq!(from, Some(client_id));
            if ch == CHANNEL_UNRELIABLE && data == b"unreliable_pos_delta" {
                has_unreliable = true;
            }
            if ch == CHANNEL_RELIABLE_ORDERED && data == b"player_spawn_event" {
                has_ordered = true;
            }
            if ch == CHANNEL_RELIABLE_UNORDERED && data == b"heartbeat_ping" {
                has_unordered = true;
            }
        }

        assert!(has_unreliable, "Channel 0 Unreliable message received");
        assert!(has_ordered, "Channel 1 ReliableOrdered message received");
        assert!(has_unordered, "Channel 2 ReliableUnordered message received");

        // 4. Server broadcasts back to client on Channel 1
        server.broadcast_message(CHANNEL_RELIABLE_ORDERED, b"welcome_player");

        for _ in 0..5 {
            server.update(dt);
            client.update(dt);
            std::thread::sleep(Duration::from_millis(5));
        }

        let cl_msgs = client.receive_messages();
        assert!(!cl_msgs.is_empty(), "Client should receive broadcast");
        assert_eq!(cl_msgs[0].1, CHANNEL_RELIABLE_ORDERED);
        assert_eq!(cl_msgs[0].2, b"welcome_player");

        // 5. Clean disconnect
        client.disconnect();
        server.disconnect();
        assert!(!client.is_connected());
        assert!(!server.is_connected());
    }

    #[test]
    fn test_rpc_encode_decode_roundtrip() {
        let packet = encode_rpc("on_damage", "[10, \"fire\"]");
        let decoded = decode_rpc(&packet).expect("RPC packet should decode");
        assert_eq!(decoded.fn_name, "on_damage");
        assert_eq!(decoded.args_json, "[10, \"fire\"]");
        assert!(decode_rpc(b"not_an_rpc").is_none());
    }
}
