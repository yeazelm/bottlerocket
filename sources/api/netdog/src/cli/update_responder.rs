use super::{error, primary_interface_name, Result};
use crate::dns::DnsSettings;
use crate::networkd_status::{get_link_status, NetworkdStatus};
use crate::{CURRENT_IP, PRIMARY_SYSCTL_CONF, SYSCTL_MARKER_FILE, SYSTEMD_SYSCTL};
use argh::FromArgs;
use snafu::{ensure, ResultExt};
use std::fmt::Write;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::process::Command;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "update-responder")]
/// Updates the various files needed when responding to events
pub(crate) struct UpdateResponderArgs {}

/// Updates the various files needed when responding to events
pub(crate) fn run() -> Result<()> {
    let primary_interface = primary_interface_name()?;

    // hardcode for on-box tests
    // let primary_interface = "enp0s16".to_string();
    // end hardcode

    let primary_link_status = get_link_status(primary_interface.clone()).unwrap();

    // This would be write_current_ip
    let primary_ip = &primary_link_status.primary_address().unwrap();
    write_current_ip(primary_ip)?;

    // Write out resolv.conf
    write_resolv_conf(&primary_link_status)?;

    // If we haven't already, set and apply default sysctls for the primary network
    // interface
    if !Path::exists(Path::new(PRIMARY_SYSCTL_CONF)) {
        write_interface_sysctl(primary_interface, PRIMARY_SYSCTL_CONF)?;
    };

    // Execute `systemd-sysctl` with our configuration file to set the sysctls
    if !Path::exists(Path::new(SYSCTL_MARKER_FILE)) {
        let systemd_sysctl_result = Command::new(SYSTEMD_SYSCTL)
            .arg(PRIMARY_SYSCTL_CONF)
            .output()
            .context(error::SystemdSysctlExecutionSnafu)?;
        ensure!(
            systemd_sysctl_result.status.success(),
            error::FailedSystemdSysctlSnafu {
                stderr: String::from_utf8_lossy(&systemd_sysctl_result.stderr)
            }
        );

        fs::write(SYSCTL_MARKER_FILE, "").unwrap_or_else(|e| {
            eprintln!(
                "Failed to create marker file {}, netdog may attempt to set sysctls again: {}",
                SYSCTL_MARKER_FILE, e
            )
        });
    }
    Ok(())
}

/// Write the default sysctls for a given interface to a given path
fn write_interface_sysctl<S, P>(interface: S, path: P) -> Result<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let interface = interface.as_ref();
    let path = path.as_ref();
    // TODO if we accumulate more of these we should have a better way to create than format!()
    // Note: The dash (-) preceding the "net..." variable assignment below is important; it
    // ensures failure to set the variable for any reason will be logged, but not cause the sysctl
    // service to fail
    // Accept router advertisement (RA) packets even if IPv6 forwarding is enabled on interface
    let ipv6_accept_ra = format!("-net.ipv6.conf.{}.accept_ra = 2", interface);
    // Enable loose mode for reverse path filter
    let ipv4_rp_filter = format!("-net.ipv4.conf.{}.rp_filter = 2", interface);

    let mut output = String::new();
    writeln!(output, "{}", ipv6_accept_ra).context(error::SysctlConfBuildSnafu)?;
    writeln!(output, "{}", ipv4_rp_filter).context(error::SysctlConfBuildSnafu)?;

    fs::write(path, output).context(error::SysctlConfWriteSnafu { path })?;
    Ok(())
}

/// Persist the current IP address to file
fn write_current_ip(ip: &IpAddr) -> Result<()> {
    fs::write(CURRENT_IP, ip.to_string())
        .context(error::CurrentIpWriteFailedSnafu { path: CURRENT_IP })
}

/// Given network status find DNS settings from the status and/or config and write the resolv.conf
fn write_resolv_conf(status: &NetworkdStatus) -> Result<()> {
    let dns_settings =
        DnsSettings::from_config_or_status(status).context(error::GetDnsSettingsSnafu)?;
    println!("{:?}", dns_settings);
    dns_settings
        .write_resolv_conf()
        .context(error::ResolvConfWriteFailedSnafu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sysctls() {
        let interface = "eno1";
        let fake_file = tempfile::NamedTempFile::new().unwrap();
        let expected = "-net.ipv6.conf.eno1.accept_ra = 2\n-net.ipv4.conf.eno1.rp_filter = 2\n";
        write_interface_sysctl(interface, &fake_file).unwrap();
        assert_eq!(std::fs::read_to_string(&fake_file).unwrap(), expected);
    }
}
