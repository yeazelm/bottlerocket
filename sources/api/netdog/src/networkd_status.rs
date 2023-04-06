//! The networkd_status module contains definitions and functions for tracking network status from networkd
//!
use crate::interface_id::InterfaceName;
use crate::NETWORKCTL;
use serde::{Deserialize, Deserializer};
use snafu::{ensure, ResultExt};
use std::cmp::Ordering;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub(crate) struct NetworkdDnsConfig {
    #[serde(rename = "Family")]
    family: u8,
    #[serde(rename = "Address", deserialize_with = "ipaddr_from_vec_de")]
    pub(crate) address: IpAddr,
    #[serde(rename = "ConfigSource")]
    config_source: String,
    #[serde(rename = "ConfigProvider", deserialize_with = "ipaddr_from_vec_de")]
    config_provider: IpAddr,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub(crate) struct SearchDomain {
    #[serde(rename = "Domain")]
    pub(crate) domain: String,
    #[serde(rename = "ConfigSource")]
    config_source: String,
    #[serde(rename = "ConfigProvider", deserialize_with = "ipaddr_from_vec_de")]
    config_provider: IpAddr,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NetworkctlIpAddr {}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub(crate) struct NetworkdStatus {
    pub(crate) name: InterfaceName,
    #[serde(rename = "DNS")]
    pub(crate) dns: Option<Vec<NetworkdDnsConfig>>,
    pub(crate) search_domains: Option<Vec<SearchDomain>>,
    #[serde(rename = "HardwareAddress")]
    pub(crate) mac_address: Vec<u8>,
    #[serde(rename = "Addresses", deserialize_with = "from_networkctl_addresses")]
    pub(crate) addresses: Vec<IpAddr>,
}

// get an IpAddr from a Vec<u8> (could be 4 or 16 length)
fn ipaddr_from_vec(address_vec: Vec<u8>) -> std::result::Result<IpAddr, Error> {
    use error::BadIpAddressSnafu;
    match address_vec.len() {
        4 => {
            let octets: [u8; 4] = address_vec.try_into().unwrap();
            Ok(IpAddr::from(octets))
        }
        16 => {
            let octets: [u8; 16] = address_vec.try_into().unwrap();
            Ok(IpAddr::from(octets))
        }
        _ => BadIpAddressSnafu {
            input: address_vec,
            msg: "invalid length, must be 4 or 16 octets".to_string(),
        }
        .fail(),
    }
}

fn ipaddr_from_vec_de<'de, D>(deserializer: D) -> std::result::Result<IpAddr, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let address_vec: Vec<u8> = Deserialize::deserialize(deserializer)?;
    ipaddr_from_vec(address_vec).map_err(D::Error::custom)
}

fn from_networkctl_addresses<'de, D>(deserializer: D) -> std::result::Result<Vec<IpAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct NetworkctlAddress {
        family: u8,
        address: Vec<u8>,
        prefix_length: u8,
        config_source: String,
    }
    let addresses: Vec<NetworkctlAddress> = Deserialize::deserialize(deserializer)?;
    let mut addrs = Vec::new();
    for addr in addresses.iter() {
        addrs.push(ipaddr_from_vec(addr.address.clone()).map_err(D::Error::custom)?);
    }
    Ok(addrs)
}

impl NetworkdStatus {
    pub(crate) fn primary_address(&self) -> Result<IpAddr> {
        use error::NoIpAddressSnafu;
        match self.addresses.len().cmp(&1) {
            Ordering::Less => NoIpAddressSnafu {
                interface: self.name.clone(),
            }
            .fail(),
            Ordering::Equal => Ok(self.addresses[0]),
            Ordering::Greater => {
                for addr in self.addresses.iter() {
                    if addr.is_ipv4() {
                        return Ok(*addr);
                    }
                }
                NoIpAddressSnafu {
                    interface: self.name.clone(),
                }
                .fail()
            }
        }
    }
}

pub(crate) fn get_link_status(link: String) -> Result<NetworkdStatus> {
    let systemd_networkctl_result = Command::new(NETWORKCTL)
        .arg("status")
        .arg("--json=pretty")
        .arg(link)
        .output()
        .context(error::NetworkctlExecutionSnafu)?;
    ensure!(
        systemd_networkctl_result.status.success(),
        error::FailedNetworkctlSnafu {
            stderr: String::from_utf8_lossy(&systemd_networkctl_result.stderr)
        }
    );

    let status_output: String = String::from_utf8(systemd_networkctl_result.stdout).unwrap();

    let networkd_status = serde_json::from_str(&status_output).unwrap();

    Ok(networkd_status)
}

mod error {
    use crate::interface_id::InterfaceId;
    use snafu::Snafu;
    use std::io;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub(crate) enum Error {
        #[snafu(display("Failed to run 'networkctl': {}", source))]
        NetworkctlExecution { source: io::Error },

        #[snafu(display("'networkctl' failed: {}", stderr))]
        FailedNetworkctl { stderr: String },

        #[snafu(display("Failed to parse IP Address: {:?} {}", input, msg))]
        BadIpAddress { input: Vec<u8>, msg: String },

        #[snafu(display("No IP Address for Primary Interface: {:?}", interface))]
        NoIpAddress { interface: InterfaceId },
    }
}

pub(crate) use error::Error;
type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    fn test_data() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data")
    }

    fn networkd_config() -> PathBuf {
        test_data().join("networkd")
    }

    fn read_output_file<P>(path: P) -> String
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let path_str = fs::read_to_string(path).unwrap();

        path_str
    }

    // full deserialize test
    #[test]
    fn parse_qemu_status() {
        let ok = networkd_config().join("qemu_networkctl_output.json");
        let network_status_str = read_output_file(ok);
        let status_output: String = String::from_utf8(network_status_str.into()).unwrap();

        // Parses correctly
        let network_status_result = serde_json::from_str::<NetworkdStatus>(&status_output);
        assert!(network_status_result.is_ok());

        // Primary address is correct for file
        let network_status = network_status_result.unwrap();
        assert_eq!(
            network_status.primary_address().unwrap(),
            Ipv4Addr::new(10, 0, 2, 15)
        );

        if let Some(nameservers) = &network_status.dns {
            let nameserver: Vec<IpAddr> = nameservers.iter().map(|n| n.address).collect();
            assert_eq!(nameserver[0], Ipv4Addr::new(10, 0, 2, 3));
        } else {
            panic!("Nameservers not found in qemu_networkctl_output.json")
        };

        assert!(network_status.search_domains.is_none());
    }

    #[test]
    fn parse_full_status() {
        let ok = networkd_config().join("full_networkctl_output.json");
        let network_status_str = read_output_file(ok);
        let status_output: String = String::from_utf8(network_status_str.into()).unwrap();

        // Parses correctly
        let network_status_result = serde_json::from_str::<NetworkdStatus>(&status_output);
        assert!(network_status_result.is_ok());

        // Primary address is correct for file
        let network_status = network_status_result.unwrap();
        assert_eq!(
            network_status.primary_address().unwrap(),
            Ipv4Addr::new(172, 31, 28, 92)
        );

        if let Some(nameservers) = &network_status.dns {
            let nameserver: Vec<IpAddr> = nameservers.into_iter().map(|n| n.address).collect();
            assert_eq!(nameserver[0], Ipv4Addr::new(172, 31, 0, 2));
        } else {
            panic!("Nameservers not found in qemu_networkctl_output.json")
        };

        if let Some(search_domains) = &network_status.search_domains {
            let search: Vec<String> = search_domains
                .into_iter()
                .map(|d| d.domain.clone())
                .collect();
            assert_eq!(search[0], "us-west-2.compute.internal".to_string());
        } else {
            panic!("Search Domains not found in qemu_networkctl_output.json")
        }
    }

    #[test]
    fn valid_ipv4addr_from_vec() {
        let ok_vec: Vec<Vec<u8>> = vec![vec![172, 1, 2, 2], vec![0, 0, 0, 0]];
        for ok in ok_vec {
            assert!(ipaddr_from_vec(ok).is_ok())
        }
    }

    #[test]
    fn valid_ip6addr_from_vec() {
        let ok_vec = vec![
            vec![254, 128, 0, 0, 0, 0, 0, 0, 80, 84, 0, 255, 254, 18, 52, 86],
            vec![254, 128, 0, 0, 0, 0, 0, 0, 4, 8, 13, 255, 254, 137, 48, 197],
        ];
        for ok in ok_vec {
            assert!(ipaddr_from_vec(ok).is_ok())
        }
    }

    #[test]
    fn invalid_ipaddr_from_vec() {
        let bad_vec = vec![
            vec![0],
            vec![1, 2, 3, 4, 5],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
        ];
        for bad in bad_vec {
            assert!(ipaddr_from_vec(bad).is_err())
        }
    }
}
