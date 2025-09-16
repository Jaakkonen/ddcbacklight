/*
CLI app that controls monitor brightness using DDC/CI protocol.
*/

use std::path::Path;

use clap::{Command, Arg};
use ddc_i2c::from_i2c_device;
use ddc::Ddc;
use mccs::Value;
use lazy_static::lazy_static;
// use lazycell::LazyCell;
// use i3ipc::I3Connection;
use swayipc::Connection;


fn value_to_current_and_max(value: Value) -> (u16, u16) {
    (value.sh as u16 * 256 + value.sl as u16, value.mh as u16 * 256 + value.ml as u16)
}

const BRIGHTNESS_VCP_CODE: u8 = 0x10;

lazy_static! {
    static ref SWAYIPC: std::sync::Mutex<Connection> = std::sync::Mutex::new(Connection::new().unwrap());
    static ref DBUS_SYSTEM: std::sync::Mutex<zbus::blocking::Connection> = std::sync::Mutex::new(zbus::blocking::Connection::system().unwrap());
}

fn get_active_output() -> String {
    SWAYIPC.lock().unwrap().get_outputs().unwrap().iter().find(|o| o.focused).unwrap().name.clone()
}

fn get_i2c_dev_by_output(output: &str) -> String {
  // Embedded display port displays don't support DDC/CI protocol.
  if output.starts_with("eDP") {
    eprintln!("Trying to set brightness of an embedded display port monitor. Aborting.");
    std::process::exit(1);
  }

  // Find the DRM output directory
  let output_path = Path::new("/sys/class/drm/").read_dir().unwrap().find(|d|
   d.as_ref().unwrap().path().to_str().unwrap().ends_with(output)
  ).map(|d| d.unwrap());

  if output_path.is_none() {
    panic!("No such output: {}", output);
  }

  let output_path = output_path.unwrap().path();

  // Try AMD GPU structure first: check for i2c-N directories
  if let Ok(entries) = output_path.read_dir() {
    for entry in entries {
      if let Ok(entry) = entry {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("i2c-") {
          let device_path = format!("/dev/{}", name);
          eprintln!("AMD GPU detected: Found I2C device {} for output {}", device_path, output);
          return device_path;
        }
      }
    }
  }

  // Fallback: try direct ddc symlink
  let ddc_symlink = output_path.join("ddc");
  if ddc_symlink.exists() {
    // AMD GPUs have a direct symlink to the i2c device
    if let Ok(target) = std::fs::read_link(&ddc_symlink) {
      // Extract i2c device number from the symlink target (e.g., "../../../i2c-7" -> "i2c-7")
      if let Some(i2c_name) = target.file_name().and_then(|n| n.to_str()) {
        let device_path = format!("/dev/{}", i2c_name);
        eprintln!("AMD GPU detected: Found I2C device {} for output {} (via ddc symlink)", device_path, output);
        return device_path;
      }
    }
  }

  // Try Intel GPU structure: ddc/i2c-dev/
  let intel_path = output_path.join("ddc").join("i2c-dev");
  if intel_path.exists() {
    if let Ok(mut entries) = intel_path.read_dir() {
      if let Some(Ok(entry)) = entries.next() {
        let dev_name = entry.file_name().to_string_lossy().to_string();
        let device_path = format!("/dev/{}", dev_name);
        eprintln!("Intel GPU detected: Found I2C device {} for output {}", device_path, output);
        return device_path;
      }
    }
  }

  panic!("Could not find I2C device for output: {}. Neither AMD nor Intel DDC structure found.", output);
}

fn set_edp_brightness(backlight_device: &str, value: u16) {
    // backlight device is something like "intel_backlight"
    // systemd-logind gives a function to set brightness level for backlight devices.
    // This doesn't require extra autentication or filesystem ACL for /sys/class/backlight devices as
    // the high-privileged systemd-logind daemon can does that.
    let dbus_system = DBUS_SYSTEM.lock().unwrap();
    let _reply = dbus_system.call_method(
        Some( "org.freedesktop.login1"),
        "/org/freedesktop/login1/session/auto",
        Some("org.freedesktop.login1.Session"),
        "SetBrightness",
        &("backlight", backlight_device, value)
    ).unwrap();
}



// impl Backend {
//     fn set_brightness(&self, value: u16) {
//         match self {
//             Backend::Backlight(backlight_device) => set_edp_brightness(backlight_device, value),
//             Backend::DdcI2c(i2c_path) => set_ddc_i2c_brightness(i2c_path, value),
//         }
//     }
// }

// enum Backend {
//     /// Backlight refers to the /sys/class/backlight device. I.e. "intel_backlight".
//     Backlight(String),
//     /// DdcI2c refers to the I2C device that supports DDC/CI protocol. I.e. "/dev/i2c-10".
//     DdcI2c(I2cDeviceDdc),
// }

fn main() {
    let matches = Command::new("monitor-brightness")
        .version("1.0")
        .about("Controls monitor brightness using DDC/CI protocol")
        .author("Jaakko Sirén <jaakko.s@iki.fi>")
        .arg(
            Arg::new("i2c_path")
                .short('i')
                .long("i2c-path")
                .required(false)
                .help("Path to the I2C device")
        )
        .subcommand(
            Command::new("get-brightness")
                .about("Get current brightness value")
        )
        .subcommand(
            Command::new("set-brightness")
                .about("Set brightness value")
                .arg(
                    Arg::new("value")
                        .required(true)
                        .help("Brightness value (0-100)")
                )
        )
        .get_matches();

    let i2c_path_maybe = matches.get_one::<String>("i2c_path");

    let i2c_path = if let Some(i2c_path) = i2c_path_maybe {
        i2c_path.to_string()
    } else {
        let active_output = get_active_output();
        get_i2c_dev_by_output(&active_output)
    };

    let mut i2c_ddc = from_i2c_device(i2c_path).unwrap();

    match matches.subcommand() {
        Some(("get-brightness", _)) => {
            let (current_value, max_value) = value_to_current_and_max(i2c_ddc.get_vcp_feature(BRIGHTNESS_VCP_CODE).unwrap());

            // Convert to percentage
            let percentage = (current_value as f32 / max_value as f32 * 100.0).round() as u16;
            println!("Current brightness: {}%", percentage);
        },
        Some(("set-brightness", sub_matches)) => {
            let value_str = sub_matches.get_one::<String>("value").unwrap();

            // Get current brightness first
            let (current_value, max_value) = value_to_current_and_max(i2c_ddc.get_vcp_feature(BRIGHTNESS_VCP_CODE).unwrap());
            let current_percentage = (current_value as f32 / max_value as f32 * 100.0).round() as i16;

            // Parse the value, handling relative changes
            let target_percentage = if value_str.starts_with('+') || value_str.starts_with('-') {
                let change = value_str.parse::<i16>()
                    .expect("Brightness change must be a number");
                let new_value = current_percentage + change;
                new_value.clamp(0, 100)
            } else {
                value_str.parse::<i16>()
                    .expect("Brightness must be a number between 0-100")
                    .clamp(0, 100)
            };

            // Convert percentage to absolute value
            let absolute_value = ((target_percentage as f32 / 100.0) * max_value as f32).round() as u16;
            i2c_ddc.set_vcp_feature(BRIGHTNESS_VCP_CODE, absolute_value).unwrap();
            println!("Brightness set to {}%", target_percentage);
        },
        _ => {
            eprintln!("Please specify either 'get-brightness' or 'set-brightness' command");
            std::process::exit(1);
        }
    }
}
