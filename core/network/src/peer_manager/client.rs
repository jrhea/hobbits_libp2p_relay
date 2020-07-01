use libp2p::identify::IdentifyInfo;
use serde::Serialize;

/// Various protocol information collected from Identify
#[derive(Clone, Debug, Serialize)]
pub struct Client {
    /// The client's name
    pub kind: ClientKind,
    /// The client's version.
    pub version: String,
    /// The OS version of the client.
    pub os_version: String,
    /// The libp2p protocol version.
    pub protocol_version: String,
    /// Identify agent string
    pub agent_string: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub enum ClientKind {
    /// A known client type
    Known(String),
    /// An unknown client.
    Unknown,
}

impl Default for Client {
    fn default() -> Self {
        Client {
            kind: ClientKind::Unknown,
            version: "unknown".into(),
            os_version: "unknown".into(),
            protocol_version: "unknown".into(),
            agent_string: None,
        }
    }
}

impl Client {
    /// Builds a `Client` from `IdentifyInfo`.
    pub fn from_identify_info(info: &IdentifyInfo) -> Self {
        let (kind, version, os_version) = client_from_agent_version(&info.agent_version);

        Client {
            kind,
            version,
            os_version,
            protocol_version: info.protocol_version.clone(),
            agent_string: Some(info.agent_version.clone()),
        }
    }
}

impl std::fmt::Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "kind:{:?} version: {}, os_version: {}",
            self.kind, self.version, self.os_version
        )
    }
}

// helper function to identify clients from their agent_version. Returns the client
// kind and it's associated version and the OS kind.
fn client_from_agent_version(agent_version: &str) -> (ClientKind, String, String) {
    let mut agent_split = agent_version.split('/');
    match agent_split.next() {
        Some(kind) => {
            if kind == "github.com" {
                let unknown = String::from("unknown");
                (
                    ClientKind::Known(kind.to_string()),
                    unknown.clone(),
                    unknown,
                )
            } else {
                let mut version = String::from("unknown");
                let mut os_version = version.clone();
                if let Some(agent_version) = agent_split.next() {
                    version = agent_version.into();
                    if let Some(agent_os_version) = agent_split.next() {
                        os_version = agent_os_version.into();
                    }
                }
                (ClientKind::Known(kind.to_string()), version, os_version)
            }
        }
        _ => {
            let unknown = String::from("unknown");
            (ClientKind::Unknown, unknown.clone(), unknown)
        }
    }
}
