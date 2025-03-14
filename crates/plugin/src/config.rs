//! Config parsing.

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serializer};
use serde_with::serde_as;
use std::net::IpAddr;
use x25519_dalek::{PublicKey, StaticSecret};

fn as_base64_privkey<S>(key: &StaticSecret, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&BASE64_STANDARD.encode(key.to_bytes()))
}

fn as_base64_pubkey<S>(key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&BASE64_STANDARD.encode(key.as_bytes()))
}

fn from_base64<'de, D, S>(deserializer: D) -> Result<S, D::Error>
where
    D: serde::Deserializer<'de>,
    S: From<[u8; 32]>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| {
            BASE64_STANDARD
                .decode(&string)
                .map_err(|err| Error::custom(err.to_string()))
        })
        .map(|bytes| TryInto::<[u8; 32]>::try_into(bytes).map(S::from).ok())
        .and_then(|opt| opt.ok_or_else(|| Error::custom("failed to deserialize public key")))
}

/// Try to parse the base64 encoded pre-shared key from the config
/// into the raw bytes it represents.
fn from_base64_opt<'de, D>(deserializer: D) -> Result<Option<[u8; 32]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match Option::<String>::deserialize(deserializer) {
        Ok(s) => match s {
            Some(s) => match BASE64_STANDARD.decode(&s) {
                Ok(b) => match b.try_into() {
                    Ok(b) => Ok(Some(b)),
                    Err(_) => Err(Error::custom("invalid pre-shared key")),
                },
                Err(e) => Err(Error::custom(e.to_string())),
            },
            None => Ok(None),
        },
        Err(e) => Err(e),
    }
}

/// A fully-parsed config
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WireGuardConfig {
    /// Local interface configuration
    pub interface: InterfaceConfig,

    /// Remote peer configuration
    pub peer: PeerConfig,
}

impl WireGuardConfig {
    /// Parse the config from the given string or return an error.
    pub fn from_str(s: &str) -> Result<WireGuardConfig, quick_xml::DeError> {
        quick_xml::de::from_str(s)
    }
}

/// Local VPN interface specific configuration
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InterfaceConfig {
    /// Our local private key
    #[serde(serialize_with = "as_base64_privkey", deserialize_with = "from_base64")]
    pub private_key: StaticSecret,

    /// Addresses to assign to local VPN interface
    pub address: Vec<IpNetwork>,

    /// DNS servers
    #[serde(default)]
    #[serde(rename = "DNS")]
    pub dns_servers: Vec<IpAddr>,

    /// DNS Search Domains
    #[serde(default)]
    #[serde(rename = "DNSSearch")]
    pub search_domains: Vec<String>,
}

/// Remote peer specific configuration
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PeerConfig {
    /// The remote endpoint's public key
    #[serde(serialize_with = "as_base64_pubkey", deserialize_with = "from_base64")]
    pub public_key: PublicKey,

    /// The port the remote endpoint is listening
    pub port: u16,

    /// The list of addresses that will get routed to the remote endpoint
    #[serde(rename = "AllowedIPs")]
    pub allowed_ips: Vec<IpNetwork>,

    /// The list of addresses that won't get routed to the remote endpoint
    #[serde(default)]
    #[serde(rename = "ExcludedIPs")]
    pub excluded_ips: Vec<IpNetwork>,

    /// The interval at which to send KeepAlive packets.
    pub persistent_keepalive: Option<u16>,

    /// An optional pre-shared key to enable an additional layer of security
    #[serde(default)]
    #[serde(deserialize_with = "from_base64_opt")]
    pub preshared_key: Option<[u8; 32]>,
}
