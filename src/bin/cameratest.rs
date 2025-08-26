use rusb::{Context, Device, DeviceHandle, UsbContext};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct UvcHelper {
    context: Arc<Context>,
    device_handle: Option<Arc<Mutex<DeviceHandle<Context>>>>,
    open_device: bool,
}

impl UvcHelper {
    pub fn new() -> Self {
        let context = Arc::new(Context::new().unwrap());
        UvcHelper {
            context,
            device_handle: None,
            open_device: false,
        }
    }

    pub fn find_and_open_camera(&mut self) -> bool {
        for device in self.context.devices().unwrap().iter() {
            if Self::is_thermal_device(&device) {
                match device.open() {
                    Ok(handle) => {
                        self.device_handle = Some(Arc::new(Mutex::new(handle)));
                        self.open_device = true;
                        println!("Thermal camera opened.");
                        return true;
                    }
                    Err(e) => {
                        println!("Failed to open device: {:?}", e);
                    }
                }
            }
        }
        false
    }

    fn is_thermal_device(device: &Device<Context>) -> bool {
        let desc = device.device_descriptor().unwrap();
        // Example vendor/product IDs, replace with actual values
        let thermal_product_ids = [22592, 14593, 22576, 22584];
        desc.class_code() == 239 && desc.sub_class_code() == 2 &&
            thermal_product_ids.contains(&desc.product_id())
    }

    pub fn stream_frames<F>(&self, mut frame_callback: F)
    where
        F: FnMut(Vec<u8>) + Send + 'static,
    {
        if let Some(handle_arc) = &self.device_handle {
            let handle_arc = handle_arc.clone();
            thread::spawn(move || {
                let endpoint = 0x81;
                let frame_size = 256 * 384;
                let mut buf = vec![0u8; frame_size];
                loop {
                    let mut handle = handle_arc.lock().unwrap();
                    match handle.read_bulk(endpoint, &mut buf, Duration::from_millis(100)) {
                        Ok(size) if size == frame_size => {
                            frame_callback(buf.clone());
                        }
                        Ok(_) => {
                            println!("Incomplete frame received");
                        }
                        Err(e) => {
                            println!("Error reading frame: {:?}", e);
                            break;
                        }
                    }
                }
            });
        }
    }
}

// Example usage
fn main() {
    let mut helper = UvcHelper::new();
    if helper.find_and_open_camera() {
        helper.stream_frames(|frame| {
            println!("Received frame of size: {}", frame.len());
            // Process frame here
        });
        // Keep main thread alive
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    } else {
        println!("No thermal camera found.");
    }
}