use std::{
    f32::consts::PI,
    sync::{Mutex, Once},
};

use esp_idf_svc::mqtt::client::EspMqttClient;
use log::info;
use smart_leds_trait::{SmartLedsWrite, RGB, RGB8};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::mqtt::send_color;

const NEO_PIXEL_PIN: u32 = 32;

#[derive(Clone, Copy)]
pub enum LedState {
    Connecting,
    Setup,
    Color(RGB8),
    PreviewColor(RGB8, RGB8)
}

pub struct LedController {
    state: LedState,
    led: Ws2812Esp32Rmt,
    tick: i32,
    mqtt: Option<EspMqttClient<'static>>
}

impl LedController {
    pub fn new() -> Result<Self, ()> {
        let led = Ws2812Esp32Rmt::new(1, NEO_PIXEL_PIN).unwrap();

        let default = Self {
            state: LedState::Connecting,
            led,
            tick: 0,
            mqtt: None
        };

        Ok(default)
    }

    pub fn get_instance() -> &'static Mutex<Self> {
        static mut INSTANCE: *const Mutex<LedController> = std::ptr::null();
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let singleton = Mutex::new(Self::new());
            unsafe {
                INSTANCE = std::mem::transmute(Box::new(singleton));
            }
        });

        unsafe { &*(INSTANCE) }
    }

    pub fn write(&mut self, state: LedState) {
        self.state = state;
        self.tick = 0;
    }

    pub fn set(state: LedState) {
        let mut lc = Self::get_instance().lock().unwrap();

        lc.write(state);
    }

    pub fn get() -> LedState {
        let mut lc = Self::get_instance().lock().unwrap();
        let state = &lc.state;
        return state.clone();
    }

    pub fn set_mqtt(mqtt: EspMqttClient<'static>) {
        let mut lc = Self::get_instance().lock().unwrap();

        lc.mqtt = Some(mqtt);
    }

    pub fn get_current_color() -> RGB8 {
        let mut lc = Self::get_instance().lock().unwrap();

        match lc.state {
            LedState::Color(color) => color,
            LedState::PreviewColor(color, _) => color,
            _ => RGB8::from((255, 255, 255))
        }
    }

    pub fn tick(&mut self) {
        let mut pixel_buffer = [RGB::from((0,0,0)); 24];

        match self.state {
            LedState::Connecting => {
                let tick = (self.tick / 4 % 24) as usize;

                let p1 = if tick == 0 { 23 } else { tick - 1 };
                let p2 = tick;
                let p3 = if tick == 23 { 0 } else { tick + 1 };

                pixel_buffer[p1] = RGB::from((0, 77, 230));
                pixel_buffer[p2] = RGB::from((25, 102, 255));
                pixel_buffer[p3] = RGB::from((0, 77, 230));
            }
            LedState::Setup => {
                let tick = (self.tick % 255) as f32 / 255.0;

                let mut brightness = 0.5 * (1.0 - f32::cos(tick * 2.0 * PI));
                brightness /= 2.0;

                let r = (25.0 * brightness).round() as u8;
                let g = (102.0 * brightness).round() as u8;
                let b = (255.0 * brightness).round() as u8;

                pixel_buffer = [RGB::new(r, g, b); 24];
            }
            LedState::Color(color) => {
                pixel_buffer = [color; 24];
            }
            LedState::PreviewColor(cur_color, preview_color) => {
                pixel_buffer = [preview_color; 24];
                if self.tick == 512 {
                    self.write(LedState::Color(cur_color));

                    if self.mqtt.is_some() {
                        let mqtt = self.mqtt.as_mut().unwrap();
                        send_color(mqtt, preview_color);
                    }
                }
            }
        }

        self.led.write(pixel_buffer.into_iter()).unwrap();
        self.tick += 1;
    }
}
