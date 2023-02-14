use std::path;

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

    /*
    This is a work in progress monolith to try different things. It exists in netdog because eventually the things
    it will do are very close to netdog. We probably need a new thing that is long living but this works to prove out
    concepts.

    The first part is mostly about trying out the low level API to dbus through the zbus library and the higher level
    Proxy macro-generated API as well.

    There is a lot of async assumptions in here, right now I'm not truly doing any async but effort will need to be put
    into switching to tokio since that is already in play at the moment. 
     */
    println!("Starting connections");
    let conn = Connection::system().await;
    eprintln!("{:?}", conn);
    let conn = conn.unwrap();
    println!("Creating manager");

    // Prove out the example of connecting to the generic Systemd DBus Manager can print things on Bottlerocket
    let proxy = SystemdManagerProxy::new(&conn).await?;
    println!("Host architecture: {}", proxy.architecture().await?);
    println!("Environment:");
    for env in proxy.environment().await? {
        println!("  {}", env);
    }

    // Try out the low level call_method functionality to prove out we can talk to Networkd DBus interfaces
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

    let links: Vec<(i32, String, OwnedObjectPath)> = reply.body()?;
    for (id, name, path) in links.iter() {
        println!("Link id: {id} Name: {name} Path: {path:?}");
    }

    // This does the same thing as the low level, but with the generated NetworkManagerProxy
    let manager = NetworkManagerProxy::new(&conn).await?;
    println!("Network manager created, getting client");
    let link_list = manager.list_links().await;
    
    // Prove out you can debug the response
    eprintln!("{:?}", link_list);
    let links = link_list.unwrap();

    // This could be done better, but an Option seemed fine for now
    let mut path_to_primary: Option<&OwnedObjectPath> = None;

    // Iterate over the links found by list_links()
    for (id, name, path) in links.iter() {
        println!("Link id: {id} Name: {name} Path: {path:?}");
        // I'm hard coding my QEMU interface but we can switch to true primary interface soon
        if name == "enp0s16" {
            // Now grab the Properties path from the listing to start watching for changes
            path_to_primary = Some(&path);
        }
    }

    // If we found the primary device, start listing to events
    if let Some(p) = path_to_primary {
        let links = zbus::fdo::PropertiesProxy::builder(&conn)
        .destination("org.freedesktop.network1")?
        .path(p)?
        .build()
        .await?;
    let mut link_props_changed = links.receive_properties_changed().await?;
    // Build a loop to just wait for events, this would be the core of a real long running system, in theory we could
    // spin up multiple async loops at once to listen on multiple things or respond to OS signals 
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
    };
    };

    Ok(())
}
