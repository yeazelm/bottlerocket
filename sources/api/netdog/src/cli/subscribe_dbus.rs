use argh::FromArgs;
use futures::stream::StreamExt;
use tokio;
use zbus::zvariant::OwnedObjectPath;
use zbus::{dbus_proxy, zvariant::ObjectPath, Connection, Result};

use zbus::MessageStream;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "subscribe-dbus", description = "foo")]
pub(crate) struct SubscribeDbusArgs {}

#[dbus_proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait SystemdManager {
    #[dbus_proxy(property)]
    fn architecture(&self) -> Result<String>;
    #[dbus_proxy(property)]
    fn environment(&self) -> Result<Vec<String>>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.network1",
    interface = "org.freedesktop.network1.Manager",
    default_path = "/org/freedesktop/network1",
    assume_defaults = false
)]
trait NetworkManager {
    #[dbus_proxy(object = "Client")]
    fn get_client(&self);

    /// Describe method
    fn describe(&self) -> zbus::Result<String>;

    /// DescribeLink method
    fn describe_link(&self, ifindex: i32) -> zbus::Result<String>;

    /// ForceRenewLink method
    fn force_renew_link(&self, ifindex: i32) -> zbus::Result<()>;

    /// GetLinkByIndex method
    fn get_link_by_index(
        &self,
        ifindex: i32,
    ) -> zbus::Result<(String, zbus::zvariant::OwnedObjectPath)>;

    /// GetLinkByName method
    fn get_link_by_name(&self, name: &str) -> zbus::Result<(i32, zbus::zvariant::OwnedObjectPath)>;

    /// ListLinks method
    fn list_links(&self) -> zbus::Result<Vec<(i32, String, zbus::zvariant::OwnedObjectPath)>>;

    /// ReconfigureLink method
    fn reconfigure_link(&self, ifindex: i32) -> zbus::Result<()>;

    /// Reload method
    fn reload(&self) -> zbus::Result<()>;

    /// RenewLink method
    fn renew_link(&self, ifindex: i32) -> zbus::Result<()>;

    /// RevertLinkDNS method
    #[dbus_proxy(name = "RevertLinkDNS")]
    fn revert_link_dns(&self, ifindex: i32) -> zbus::Result<()>;

    /// RevertLinkNTP method
    #[dbus_proxy(name = "RevertLinkNTP")]
    fn revert_link_ntp(&self, ifindex: i32) -> zbus::Result<()>;

    /// SetLinkDNS method
    #[dbus_proxy(name = "SetLinkDNS")]
    fn set_link_dns(&self, ifindex: i32, addresses: &[(i32, &[u8])]) -> zbus::Result<()>;

    /// SetLinkDNSEx method
    #[dbus_proxy(name = "SetLinkDNSEx")]
    fn set_link_dnsex(
        &self,
        ifindex: i32,
        addresses: &[(i32, &[u8], u16, &str)],
    ) -> zbus::Result<()>;

    /// SetLinkDNSOverTLS method
    #[dbus_proxy(name = "SetLinkDNSOverTLS")]
    fn set_link_dnsover_tls(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;

    /// SetLinkDNSSEC method
    #[dbus_proxy(name = "SetLinkDNSSEC")]
    fn set_link_dnssec(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;

    /// SetLinkDNSSECNegativeTrustAnchors method
    #[dbus_proxy(name = "SetLinkDNSSECNegativeTrustAnchors")]
    fn set_link_dnssecnegative_trust_anchors(
        &self,
        ifindex: i32,
        names: &[&str],
    ) -> zbus::Result<()>;

    /// SetLinkDefaultRoute method
    fn set_link_default_route(&self, ifindex: i32, enable: bool) -> zbus::Result<()>;

    /// SetLinkDomains method
    fn set_link_domains(&self, ifindex: i32, domains: &[(&str, bool)]) -> zbus::Result<()>;

    /// SetLinkLLMNR method
    #[dbus_proxy(name = "SetLinkLLMNR")]
    fn set_link_llmnr(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;

    /// SetLinkMulticastDNS method
    #[dbus_proxy(name = "SetLinkMulticastDNS")]
    fn set_link_multicast_dns(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;

    /// SetLinkNTP method
    #[dbus_proxy(name = "SetLinkNTP")]
    fn set_link_ntp(&self, ifindex: i32, servers: &[&str]) -> zbus::Result<()>;

    /// AddressState property
    #[dbus_proxy(property)]
    fn address_state(&self) -> zbus::Result<String>;

    /// CarrierState property
    #[dbus_proxy(property)]
    fn carrier_state(&self) -> zbus::Result<String>;

    /// IPv4AddressState property
    #[dbus_proxy(property, name = "IPv4AddressState")]
    fn ipv4_address_state(&self) -> zbus::Result<String>;

    /// IPv6AddressState property
    #[dbus_proxy(property, name = "IPv6AddressState")]
    fn ipv6_address_state(&self) -> zbus::Result<String>;

    /// NamespaceId property
    #[dbus_proxy(property)]
    fn namespace_id(&self) -> zbus::Result<u64>;

    /// OnlineState property
    #[dbus_proxy(property)]
    fn online_state(&self) -> zbus::Result<String>;

    /// OperationalState property
    #[dbus_proxy(property)]
    fn operational_state(&self) -> zbus::Result<String>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.Network1",
    interface = "org.freedesktop.Network1.Client"
)]
trait Client {
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;

    #[dbus_proxy(property)]
    fn set_desktop_id(&mut self, id: &str) -> Result<()>;
}

pub(crate) async fn run() -> Result<()> {
    println!("Starting connections");
    println!("try1");
    let conn = Connection::system().await;
    eprintln!("{:?}", conn);
    let conn = conn.unwrap();
    println!("Creating manager");
    let proxy = SystemdManagerProxy::new(&conn).await?;
    println!("Host architecture: {}", proxy.architecture().await?);
    println!("Environment:");
    for env in proxy.environment().await? {
        println!("  {}", env);
    }

    let reply = conn
        .call_method(
            Some("org.freedesktop.network1"),
            "/org/freedesktop/network1",
            Some("org.freedesktop.network1.Manager"),
            "ListLinks",
            &(),
        )
        .await?;

    println!("Called, printing body next");
    // let names: Vec<()> = reply.body()?;
    let links: Vec<(i32, String, OwnedObjectPath)> = reply.body()?;
    // for name in names.iter() {
    //     println!("{}", name);
    // }
    for (id, name, path) in links.iter() {
        println!("Link id: {id} Name: {name} Path: {path:?}");
    }

    let manager = NetworkManagerProxy::new(&conn).await?;
    println!("Network manager created, getting client");
    let link_list = manager.list_links().await;

    eprintln!("{:?}", link_list);
    let links = link_list.unwrap();

    let mut primary: &i32 = &0;
    let mut path_to_primary: Option<&OwnedObjectPath> = None;
    for (id, name, path) in links.iter() {
        println!("Link id: {id} Name: {name} Path: {path:?}");
        if name == "eth0" {
            primary = id;
            path_to_primary = Some(&path);
        };
    }
    //let mut client = manager.get_client().await?;
    println!("built client");
    // Set the client for connecting to dbus
    //client.set_desktop_id("org.freedesktop.zbus").await?;

    let dest = format!("org.freedesktop.network1/link/{}", primary);
    println!("{}", dest);

    if let Some(p) = path_to_primary {
        let links = zbus::fdo::PropertiesProxy::builder(&conn)
            .destination("org.freedesktop.network1")?
            .path(p)?
            .build()
            .await?;
        let mut link_props_changed = links.receive_properties_changed().await?;
        while let Some(signal) = link_props_changed.next().await {
            let args = signal.args()?;

            for (name, value) in args.changed_properties().iter() {
                println!(
                    "{}.{} changed to `{:?}`",
                    args.interface_name(),
                    name,
                    value
                );
            }
        }
    };

    //let links = zbus::fdo::PropertiesProxy::builder(&conn)
    //    .destination("org.freedesktop.network1")?
    //    .path(&links[1].2)?
    //    .build()
    //    .await?;
    //let mut link_props_changed = links.receive_properties_changed().await?;

    //client.start().await?;

    // let links = link_list.clone();

    // for (id, name, path) in links.iter() {
    //     println!("Link id: {id} Name: {name} Path: {path:?}");
    // }

    //while let Some(signal) = link_props_changed.next().await {
    //    let args = signal.args()?;

    //    for (name, value) in args.changed_properties().iter() {
    //        println!(
    //            "{}.{} changed to `{:?}`",
    //            args.interface_name(),
    //            name,
    //            value
    //        );
    //    }
    //}
    // tokio::try_join!(
    //     async {
    //         while let Some(signal) = link_props_changed.next().await {
    //             let args = signal.args()?;

    //             for (name, value) in args.changed_properties().iter() {
    //                 println!("{}.{} changed to `{:?}`", args.interface_name(), name, value);
    //             }
    //         }
    //         Ok::<(), zbus::Error>(())
    //     }

    // )?;

    Ok(())
}
