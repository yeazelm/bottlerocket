//! The networkd_status module contains definitions and functions for tracking network status from networkd
//!
//!
//!
//!
use crate::interface_id::InterfaceName;
use crate::NETWORKCTL;
use serde::{Deserialize, Deserializer};
use snafu::ensure;
use snafu::ResultExt;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct NetworkdDnsConfig {
    #[serde(rename = "Family")]
    family: u8,
    #[serde(rename = "Address", deserialize_with = "ipaddr_from_vec")]
    pub(crate) address: IpAddr,
    #[serde(rename = "ConfigSource")]
    config_source: String,
    #[serde(rename = "ConfigProvider", deserialize_with = "ipaddr_from_vec")]
    config_provider: IpAddr,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SearchDomain {
    #[serde(rename = "Domain")]
    pub(crate) domain: String,
    #[serde(rename = "ConfigSource")]
    config_source: String,
    #[serde(rename = "ConfigProvider", deserialize_with = "ipaddr_from_vec")]
    config_provider: IpAddr,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NetworkctlIpAddr {}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct NetworkdStatus {
    pub(crate) name: InterfaceName,
    #[serde(rename = "DNS")]
    pub(crate) dns: Option<Vec<NetworkdDnsConfig>>,
    pub(crate) search_domains: Option<Vec<SearchDomain>>,
    #[serde(rename = "HardwareAddress")]
    pub(crate) mac_address: Vec<u8>,
    #[serde(rename = "IPv6LinkLocalAddress")]
    pub(crate) ipv6_local_address: Option<Vec<u8>>, // TODO: This should be an actual IP address... thanks networkctl
    #[serde(rename = "Addresses", deserialize_with = "from_networkctl_addresses")]
    pub(crate) addresses: Vec<IpAddr>,
}

// get an IpAddr from a Vec<u8> (could be 4 or 16 length)
fn ipaddr_from_vec<'de, D>(deserializer: D) -> std::result::Result<IpAddr, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let address_vec: Vec<u8> = Deserialize::deserialize(deserializer)?;
    match address_vec.len() {
        4 => {
            let octets: [u8; 4] = address_vec.clone().try_into().unwrap();
            Ok(IpAddr::from(octets))
        }
        16 => {
            let octets: [u8; 16] = address_vec.clone().try_into().unwrap();
            Ok(IpAddr::from(octets))
        }
        _ => return Err(D::Error::custom("Could not match IpAddr")),
    }
}

fn from_networkctl_addresses<'de, D>(deserializer: D) -> std::result::Result<Vec<IpAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct NetworkctlAddress {
        family: u8,
        address: Vec<u8>,
        prefix_length: u8,
        config_source: String,
    }
    let addresses: Vec<NetworkctlAddress> = Deserialize::deserialize(deserializer)?;
    let mut addrs = Vec::new();
    for addr in addresses.iter() {
        match addr.family {
            2 => {
                if addr.address.len() == 4 {
                    let octets: [u8; 4] = addr.address.clone().try_into().unwrap();
                    addrs.push(IpAddr::from(octets))
                }
            }
            10 => {
                if addr.address.len() == 16 {
                    let octets: [u8; 16] = addr.address.clone().try_into().unwrap();
                    addrs.push(IpAddr::from(octets))
                }
            }
            _ => {}
        }
    }
    Ok(addrs)
}

impl NetworkdStatus {
    pub(crate) fn primary_address(&self) -> IpAddr {
        if self.addresses.len() > 1 {
            for addr in self.addresses.iter() {
                if addr.is_ipv4() {
                    return addr.clone();
                }
            }
        } else if self.addresses.len() < 1 {
            // TODO: Find a better way to return something or switch to an Option for the function
            return std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        }
        self.addresses[0].clone()
    }
}

pub(crate) fn get_link_status(link: String) -> Result<NetworkdStatus> {
    let systemd_networkctl_result = Command::new(NETWORKCTL)
        .arg("status").arg("--json=pretty").arg(link)
        .output()
        .context(error::NetworkctlExecutionSnafu)?;
    ensure!(
        systemd_networkctl_result.status.success(),
        error::FailedNetworkctlSnafu {
            stderr: String::from_utf8_lossy(&systemd_networkctl_result.stderr)
        }
    );
    // Temporary read of file for testing, this will turn into a unit test
    // use std::fs;
    // let status_output = fs::read_to_string(
    //     "/home/fedora/git/bottlerocket/sources/api/netdog/qemu_networkctl_output.json",
    // )
    // .expect("Couldn't read the file!");

    let status_output: String = String::from_utf8(systemd_networkctl_result.stdout).unwrap();

    let networkd_status = serde_json::from_str(&status_output).unwrap();

    println!("{:#?}", networkd_status);

    Ok(networkd_status)
}

mod error {
    use snafu::Snafu;
    use std::io;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub(crate) enum Error {
        #[snafu(display("Failed to run 'networkctl': {}", source))]
        NetworkctlExecution { source: io::Error },

        #[snafu(display("'networkctl' failed: {}", stderr))]
        FailedNetworkctl { stderr: String },
    }
}

pub(crate) use error::Error;
type Result<T> = std::result::Result<T, error::Error>;
