use std::{error::Error, sync::{Mutex, Once}};

use embedded_svc::ipv4::{DHCPClientSettings, RouterConfiguration};
use esp_idf_svc::{nvs::{EspNvs, NvsDefault, EspDefaultNvsPartition, EspDefaultNvs}, wifi::{EspWifi, BlockingWifi}, netif::{NetifConfiguration, EspNetif, NetifStack}, ipv4};

pub const NVS_SSID: &'static str = "ssid";
pub const NVS_PASS: &'static str = "pass";
pub const NVS_BRIGHTNESS: &'static str = "brns";

pub fn get_default_nvs() -> &'static Mutex<EspDefaultNvs> {
    static mut INSTANCE: *const Mutex<EspDefaultNvs> = std::ptr::null();
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        let default_partition = EspDefaultNvsPartition::take().expect("Failed to take default nvs partition");
        let nvs = EspDefaultNvs::new(default_partition, "lamp", true).unwrap();
        let singleton = Mutex::new(nvs);
        unsafe {
            INSTANCE = std::mem::transmute(Box::new(singleton));
        }
    });

    unsafe { &*(INSTANCE) }
}



pub fn swap_netif(wifi: &mut EspWifi, device_name: &str) {
    wifi.swap_netif(EspNetif::new_with_conf(&NetifConfiguration {
        key: format!("{}_{}", device_name, "sta").as_str().into(),
        description: "lamp".into(),
        route_priority: 1,
        ip_configuration: ipv4::Configuration::Client(
            ipv4::ClientConfiguration::DHCP(
                DHCPClientSettings {
                    hostname: Some(device_name.into())
                }
            )
        ),
        stack: NetifStack::Sta,
        custom_mac: None,
    }).unwrap(), EspNetif::new_with_conf(&NetifConfiguration {
        key: format!("{}_{}", device_name, "ap").as_str().into(),
        description: "lamp".into(),
        route_priority: 0,
        ip_configuration: ipv4::Configuration::Router(
            RouterConfiguration::default()
        ),
        stack: NetifStack::Ap,
        custom_mac: None,
    }).unwrap());
}
