use std::{net::Ipv4Addr, str::FromStr, error::Error, sync::{Mutex, Once, Arc}, time::Duration, thread};

use anyhow::{bail, Result};
use embedded_svc::wifi::AccessPointConfiguration;
use esp_idf_svc::{
    eventloop::{EspSystemEventLoop, EspEventLoop, System},
    hal::{peripheral, modem::Modem},
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi}, netif::{EspNetif, NetifConfiguration, NetifStack}, ipv4::{self, DHCPClientSettings, RouterConfiguration, Subnet, Mask},
};
use log::{info, error};

use crate::CONFIG;

static mut INSTANCE: *const Mutex<EspWifi> = std::ptr::null();
static ONCE: Once = Once::new();


pub fn set_wifi(modem: Modem, sysloop: EspEventLoop<System>) -> &'static Mutex<EspWifi<'static>> {

    ONCE.call_once(|| {
        info!("Set wifi");
        let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), None).expect("Failed to construct wifi!");
        info!("Wrapped WiFi");
        let singleton = Mutex::new(esp_wifi);
        unsafe {
            INSTANCE = std::mem::transmute(Box::new(singleton));
        }
    });

    unsafe { &*(INSTANCE) }
}

pub fn get_wifi() -> &'static Mutex<EspWifi<'static>> {
    unsafe { &*(INSTANCE) }
}


pub fn connect(
    ssid: &str,
    pass: &str,
    reconfig: bool
) -> Result<bool, Box<dyn Error>> {
    let mut wifi = get_wifi().lock()?;
    let mut auth_method = AuthMethod::WPA2Personal;

    let ssid = ssid.replace("+", " ");
    let ssid = ssid.as_str();

    if pass.is_empty() {
        auth_method = AuthMethod::None;
        info!("Wifi password is empty");
    }

    if reconfig {
        wifi.set_configuration(&embedded_svc::wifi::Configuration::Mixed(ClientConfiguration::default(), AccessPointConfiguration {
            ssid: CONFIG.device_name.into(),
            ..Default::default()
        }))?;
    }


    info!("Starting wifi...");

    wifi.start()?;

    info!("Scanning...");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| {
        a.ssid == ssid
    });

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    info!("Setting config...");

    let res = wifi.set_configuration(&Configuration::Mixed(ClientConfiguration {
        ssid: ssid.into(),
        password: pass.into(),
        channel,
        auth_method,
        ..Default::default()
    }, AccessPointConfiguration {
        ssid: CONFIG.device_name.into(),
        ..Default::default()
    }));

    if res.is_err() {
        let e = res.unwrap_err();
        e.panic();
        error!("{:?}", e);
        return Err(res.unwrap_err().into());
    }

    info!("Connecting wifi...");

    wifi.connect()?;

    info!("Waiting for DHCP lease...");

    while !wifi.sta_netif().is_up()? {
        thread::sleep(Duration::from_millis(100));
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    let connected = ip_info.ip.to_string() != "0.0.0.0".to_string();
    info!("Connected? {}", connected);

    Ok(connected)
}
