//! The dns module contains the code necessary to gather DNS settings from config file,
//! supplementing with DHCP lease if it exists.  It also contains the code necessary to write a
//! properly formatted `resolv.conf`.
use crate::lease::LeaseInfo;
use crate::networkd_status::NetworkdStatus;
use crate::RESOLV_CONF;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use snafu::ResultExt;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::net::IpAddr;
use std::path::Path;

static DNS_CONFIG: &str = "/etc/netdog.toml";

#[derive(Default, Debug, Deserialize, PartialEq)]
pub(crate) struct DnsSettings {
    #[serde(rename = "name-servers")]
    nameservers: Option<BTreeSet<IpAddr>>,
    #[serde(rename = "search-list")]
    search: Option<Vec<String>>,
}

impl DnsSettings {
    /// Create a DnsSettings from TOML config file, supplementing missing settings with settings
    /// from DHCP lease if provided.  (In the case of static addressing, a DHCP lease won't exist)
    pub(crate) fn from_config_or_lease(lease: Option<&LeaseInfo>) -> Result<Self> {
        let mut settings = Self::from_config()?;
        if let Some(lease) = lease {
            settings.merge_lease(lease);
        }
        Ok(settings)
    }

    /// Merge missing DNS settings into `self` using DHCP lease
    fn merge_lease(&mut self, lease: &LeaseInfo) {
        if self.nameservers.is_none() {
            self.nameservers = lease.dns_servers.clone();
        }

        if self.search.is_none() {
            self.search = lease.dns_search.clone()
        }
    }

    /// Create a DnsSettings from TOML config file, supplementing missing settings from data in
    /// the NetworkdStatus.
    pub(crate) fn from_config_or_status(status: &NetworkdStatus) -> Result<Self> {
        let mut settings = Self::from_config()?;
        settings.merge_status(status);
        Ok(settings)
    }

    fn merge_status(&mut self, status: &NetworkdStatus) {
        // This is probably actually a Vec of DNS configs?
        if self.nameservers.is_none() {
            if let Some(dns_nameservers) = &status.dns {
                self.nameservers = Some(dns_nameservers.iter().map(|n| n.address).collect());
            }
        }

        if self.search.is_none() {
            if let Some(search_domains) = &status.search_domains {
                self.search = Some(search_domains.iter().map(|d| d.domain.clone()).collect());
            }
        }
    }

    /// Create a DnsSettings from TOML config file
    fn from_config() -> Result<Self> {
        Self::from_config_impl(DNS_CONFIG)
    }

    fn from_config_impl<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        // Ensure we don't attempt to load a nonexistent or empty config file.  At boot time, the
        // config file won't exist because it hasn't been generated by the API yet.  After that,
        // the file will always exist because it's a configuration file for settings in the API and
        // will always be generated.  If the DNS settings aren't populated, the file will be empty.
        // We can assume if the file is empty that the settings don't exist.
        let path = path.as_ref();
        let config_exists = if Path::exists(path) {
            let file_len = fs::metadata(path)
                .context(error::DnsConfMetaSnafu { path })?
                .len();
            file_len != 0
        } else {
            false
        };

        if config_exists {
            let config_str =
                fs::read_to_string(path).context(error::DnsConfReadFailedSnafu { path })?;
            let dns_config =
                toml::from_str(&config_str).context(error::DnsConfParseSnafu { path })?;

            Ok(dns_config)
        } else {
            eprintln!("No DNS configuration exists in {}", DNS_CONFIG);
            Ok(DnsSettings::default())
        }
    }

    /// Write resolver configuration for libc.
    pub(crate) fn write_resolv_conf(&self) -> Result<()> {
        Self::write_resolv_conf_impl(self, RESOLV_CONF)
        // Self::write_resolv_conf_impl(self, "/tmp/resolv.conf")
    }

    fn write_resolv_conf_impl<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let mut output = String::new();

        if let Some(s) = &self.search {
            writeln!(output, "search {}", s.join(" "))
                .context(error::ResolvConfBuildFailedSnafu)?;
        }

        if let Some(nameservers) = &self.nameservers {
            // Randomize name server order, for libc implementations like musl that send
            // queries to the first N servers.
            let mut dns_servers: Vec<IpAddr> = nameservers.clone().into_iter().collect();
            dns_servers.shuffle(&mut thread_rng());
            for n in dns_servers {
                writeln!(output, "nameserver {}", n).context(error::ResolvConfBuildFailedSnafu)?;
            }
        }

        fs::write(path, output).context(error::ResolvConfWriteFailedSnafu { path })
    }
}

mod error {
    use snafu::Snafu;
    use std::io;
    use std::path::PathBuf;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub(crate) enum Error {
        #[snafu(display("Failed to read DNS settings from '{}': {}", path.display(), source))]
        DnsConfReadFailed { path: PathBuf, source: io::Error },

        #[snafu(display("Failed to read file metadata from '{}': {}", path.display(), source))]
        DnsConfMeta {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to parse DNS settings from '{}': {}", path.display(), source))]
        DnsConfParse {
            path: PathBuf,
            source: toml::de::Error,
        },

        #[snafu(display("Failed to build resolver configuration: {}", source))]
        ResolvConfBuildFailed { source: std::fmt::Error },

        #[snafu(display("Failed to write resolver configuration to '{}': {}", path.display(), source))]
        ResolvConfWriteFailed { path: PathBuf, source: io::Error },
    }
}

pub(crate) use error::Error;
type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_data() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("dns")
    }

    #[test]
    fn dns_from_config() {
        let config = test_data().join("netdog.toml");
        let dns_settings = DnsSettings::from_config_impl(&config).unwrap();
        assert!(dns_settings.nameservers.is_some());
        assert!(dns_settings.search.is_some());
    }

    #[test]
    fn empty_config() {
        let empty = tempfile::NamedTempFile::new().unwrap();
        let dns_settings = DnsSettings::from_config_impl(&empty).unwrap();
        assert!(dns_settings.nameservers.is_none());
        assert!(dns_settings.search.is_none());
    }

    #[test]
    fn missing_config() {
        let missing = "/a/nonexistent/net/config/path";
        let dns_settings = DnsSettings::from_config_impl(&missing).unwrap();
        assert!(dns_settings.nameservers.is_none());
        assert!(dns_settings.search.is_none());
    }

    #[test]
    fn dns_from_lease_file() {
        let lease_path = test_data().join("leaseinfo.eth0.dhcp.ipv4");
        let lease = LeaseInfo::from_lease(&lease_path).unwrap();
        let mut got = DnsSettings::default();
        got.merge_lease(&lease);

        let mut nameservers = BTreeSet::new();
        nameservers.insert("192.168.0.2".parse::<IpAddr>().unwrap());
        let search = Some(vec!["us-west-2.compute.internal".to_string()]);
        let expected = DnsSettings {
            nameservers: Some(nameservers),
            search,
        };

        assert_eq!(got, expected)
    }

    #[test]
    fn write_resolv_conf_from_lease_single_nameserver() {
        let lease_path = test_data().join("leaseinfo.eth0.dhcp.ipv4");
        let lease = LeaseInfo::from_lease(&lease_path).unwrap();

        let fake_file = tempfile::NamedTempFile::new().unwrap();
        let mut settings = DnsSettings::default();
        settings.merge_lease(&lease);
        settings.write_resolv_conf_impl(&fake_file).unwrap();

        let expected = "search us-west-2.compute.internal\nnameserver 192.168.0.2\n";
        assert_eq!(std::fs::read_to_string(&fake_file).unwrap(), expected);
    }

    #[test]
    fn write_resolv_conf_from_lease_multiple_nameservers() {
        let lease_path = test_data().join("leaseinfo.eth0.dhcp.ipv4.multiple-dns");
        let lease = LeaseInfo::from_lease(&lease_path).unwrap();

        let fake_file = tempfile::NamedTempFile::new().unwrap();
        let mut settings = DnsSettings::default();
        settings.merge_lease(&lease);
        settings.write_resolv_conf_impl(&fake_file).unwrap();

        // Since we shuffle the nameservers, it's possible for the resulting file to be either of
        // the following
        let format1 =
            "search us-west-2.compute.internal\nnameserver 192.168.0.2\nnameserver 1.2.3.4\n";
        let format2 =
            "search us-west-2.compute.internal\nnameserver 1.2.3.4\nnameserver 192.168.0.2\n";

        // The resulting file must be either format 1 or 2
        let resolv_conf = std::fs::read_to_string(&fake_file).unwrap();
        assert_ne!(resolv_conf == format1, resolv_conf == format2)
    }

    #[test]
    fn write_resolv_conf_from_config_multiple_nameservers() {
        let fake_file = tempfile::NamedTempFile::new().unwrap();
        let config = test_data().join("netdog.toml");
        let settings = DnsSettings::from_config_impl(config).unwrap();
        settings.write_resolv_conf_impl(&fake_file).unwrap();

        // Since we shuffle the nameservers, it's possible for the resulting file to be either of
        // the following
        let format1 = "search us-west-2.compute.internal foo.bar.baz\nnameserver 1.2.3.4\nnameserver 2.3.4.5\n";
        let format2 = "search us-west-2.compute.internal foo.bar.baz\nnameserver 2.3.4.5\nnameserver 1.2.3.4\n";

        // The resulting file must be either format 1 or 2
        let resolv_conf = std::fs::read_to_string(&fake_file).unwrap();
        assert_ne!(resolv_conf == format1, resolv_conf == format2)
    }
}
