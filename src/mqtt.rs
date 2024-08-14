use std::sync::{Mutex, Once};

use embedded_svc::mqtt::{self, client::QoS};
use esp_idf_svc::mqtt::client::{EspMqttMessage, MqttClientConfiguration, EspMqttClient, Event::Received};
use log::info;
use rgb_led::RGB8;

use crate::{led_controller::{LedController, LedState}, CONFIG, UUID};

pub fn color_topic(uuid: &str) -> String {
    format!("{}/color", uuid)
}

pub fn process_message(msg: &EspMqttMessage) {
    match msg.details() {
        // All messages in this exercise will be of type `Complete`
        // The other variants of the `Details` enum are for larger message payloads
        details => {
    
            // Cow<&[u8]> can be coerced into a slice &[u8] or a Vec<u8>
            // You can coerce it into a slice to be sent to try_from()
            let message_data: &[u8] = &msg.data();
            let topic = msg.topic().unwrap().split("/").last().unwrap();

            if topic == "color" {
                let data = std::str::from_utf8(message_data).unwrap().split(",").collect::<Vec<&str>>();
                let partner = data[0];
                let color = data[1].parse::<u32>().unwrap();
                let r = ((color & 0xff0000) >> 16) as u8;
                let g = ((color & 0x00ff00) >> 8) as u8;
                let b = (color & 0x0000ff) as u8;
                let color = RGB8::new(r,g,b);

                if partner == CONFIG.device_name {
                    match LedController::get() {
                        LedState::Color(_) => LedController::set(LedState::Color(color)),
                        LedState::PreviewColor(cur_color, preview_color) => LedController::set(LedState::PreviewColor(color, preview_color)),
                        _ => {}
                    }
                }
            }
        }
        // Use Rust Analyzer to generate the missing match arms or match an incomplete message with a log message.
    }
}


pub fn send_color(mqtt: &mut EspMqttClient, color: RGB8) {
    
    let decimal = ((color.r as i32) << 16_i32) + ((color.g as i32) << 8_i32) + (color.b as i32);
    info!("{decimal}");
    let payload = format!("{},{}", CONFIG.partner_name, decimal);
    info!("{payload}");
    let payload = payload.as_bytes();

    let res = mqtt.publish(color_topic(UUID).as_str(), QoS::AtLeastOnce, true, payload);

    if res.is_err() {
        info!("Failed to publish!")
    } else {
        info!("payload sent")
    }
}