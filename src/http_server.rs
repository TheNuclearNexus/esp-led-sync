use core::str;
use embedded_svc::{
    http::{server::Request, Method},
    io::Write,
};
use smart_leds_trait::{RGB, RGBW, RGBA, White};
use std::{collections::HashMap, error::Error, sync::{RwLock, Arc}, process, thread::{spawn, self}, time::Duration};

use esp_idf_svc::{http::{self, client::EspHttpConnection, server::EspHttpServer}, eventloop::{System, EspSystemEventLoop}};

use log::info;

use crate::{led_controller::{LedController, LedState}, utils::{get_default_nvs, NVS_SSID, NVS_PASS}, wifi::{self, get_wifi}};

fn read_payload(payload: &str) -> Result<HashMap<&str, &str>, Box<dyn Error>> {
    let mut body = HashMap::new();

    payload.split("&").for_each(|q| {
        let parts: Vec<&str> = q.split("=").collect();
        let key = parts[0];
        let value = parts[1];

        body.insert(key, value);
    });

    Ok(body)
}

pub fn init() -> Result<EspHttpServer<'static>, Box<dyn Error>> {
    // 1.Create a `EspHttpServer` instance using a default configuration
    let mut server = EspHttpServer::new(&http::server::Configuration {
        http_port: 3030,
        ..Default::default()
    })?;

    // 2. Write a handler that returns the index page
    server.fn_handler("/", Method::Get, |request| {
        let mut response = request.into_ok_response()?;
        response.write_all(&index().as_bytes())?;
        Ok(())
    })?;

    server.fn_handler("/color", Method::Get, |request| {
        let mut response = request.into_ok_response()?;
        response.write_all(&color().as_bytes())?;
        Ok(())
    })?;

    server.fn_handler("/setup", Method::Get, |request| {
        let mut response = request.into_ok_response()?;
        response.write_all(&setup().as_bytes())?;
        Ok(())
    })?;


    server.fn_handler("/wifi-settings", Method::Post, |mut request| {
        let buf: &mut [u8] = &mut [0_u8; 256];
        let size = request.read(buf)?;
        request.read(buf)?;
        let payload = str::from_utf8(&(buf[0..size]))?;
        let body = read_payload(payload).unwrap();


        let ssid = *body.get("ssid").unwrap_or(&"");
        let password = *body.get("password").unwrap_or(&"");

        info!("SSID: {ssid}\nPass: {password}");

        let mut response = request.into_ok_response()?;


        let mut nvs = get_default_nvs().lock().unwrap();

        nvs.set_str(NVS_SSID, ssid).expect("failed to write ssid!");
        nvs.set_str(NVS_PASS, password).expect("failed to write password!");
        info!("Wrote to NVS");

        spawn(|| { 
            thread::sleep(Duration::from_millis(50));
            process::exit(1);
        });
        
        response.write_all(&r#""#.as_bytes())?;
        Ok(())
    })?;

    server.fn_handler("/available-networks", Method::Get, |mut request| {
        let mut wifi = get_wifi().lock().unwrap();
        
        let networks = wifi.scan()?;

        let template = include_str!("../html/templates/network-option.html");

        let json = networks.iter().map(|n| 
            template.replace("{% ssid %}", n.ssid.as_str())
        ).collect::<Vec<String>>().join("\n");
        
        let mut response = request.into_ok_response()?;
        response.write_all(&json.as_bytes())?;
        Ok(())
    })?;

    server.fn_handler("/set-color", Method::Post, |mut request| {
        let buf: &mut [u8] = &mut [0_u8; 16];
        let size = request.read(buf)?;
        request.read(buf)?;
        let payload = str::from_utf8(&(buf[0..size]))?;
        let body = read_payload(payload).unwrap();
        
        let color = body.get("color").unwrap_or(&"0").parse::<u32>()?;
        let r = ((color & 0xff0000) >> 16) as u8;
        let g = ((color & 0x00ff00) >> 8) as u8;
        let b = (color & 0x0000ff) as u8;

        let cur_color = LedController::get_current_color();

        LedController::set(LedState::PreviewColor(cur_color, RGB::from((r, g, b))));

        let mut response = request.into_ok_response()?;
        response.write_all(&"".as_bytes())?;
        Ok(())
    })?;

    Ok(server)
}

fn template(mut html: String, context: HashMap<String, String>) -> String {
    for (key, value) in context {
        html = html.replace(format!("{{% {key} %}}").as_str(), value.as_str());
    }

    html
}

fn inject_to_body(content: String) -> String {
    let index = include_str!("../html/base.html");
    let index = index.replace("{% content %}", content.as_str());
    
    index
}

fn index() -> String {
    let ip_info = get_wifi().lock().unwrap().sta_netif().get_ip_info().unwrap();

    let connected = ip_info.ip.to_string() != "0.0.0.0".to_string();

    let context = HashMap::from([
        ("is_setup".to_string(), connected.to_string()),
        ("ip".to_string(), ip_info.ip.to_string())
    ]);
    let html = include_str!("../html/index.html");
    let html = template(html.to_string(), context);
    inject_to_body(html)
}

fn color() -> String {
    let cur_color = LedController::get_current_color();
    let context = HashMap::from([
        ("current_color".to_string(), format!("rgb({}, {}, {})", cur_color.r, cur_color.g, cur_color.b))
    ]);
    let html = include_str!("../html/color.html");
    let html = template(html.to_string(), context);
    inject_to_body(html)
}

fn setup() -> String {
    let html = include_str!("../html/setup.html");
    inject_to_body(html.to_string())
}


