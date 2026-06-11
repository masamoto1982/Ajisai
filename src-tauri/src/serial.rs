//! Native serial backend for the Tauri desktop channel (Phase 3).
//!
//! Mirrors the Web Serial adapter's contract (see SPECIFICATION.html §9.4 and
//! `docs/dev/web-serial-module-design.md`): the Ajisai WASM core stays
//! platform-agnostic and only emits `SERIAL:` commands / drains an injected
//! inbox. Here those commands are fulfilled with the `serialport` crate, and a
//! per-port reader thread pushes received bytes to the frontend as `serial-rx`
//! events so the TypeScript adapter can accumulate them for snapshot injection.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// An open port: the handle used for writes, plus a stop flag shared with its
/// reader thread so `serial_close` can end the thread deterministically.
struct OpenPort {
    port: Box<dyn serialport::SerialPort>,
    stop: Arc<AtomicBool>,
}

/// Tauri-managed state holding every open port keyed by its device name.
#[derive(Default)]
pub struct SerialState(Mutex<HashMap<String, OpenPort>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerialRx {
    port_id: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerialDisconnect {
    port_id: String,
}

const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// Cap a single `serial_write` payload at 1 MiB. Matches `MAX_WRITE_BYTES`
/// in `src/platform/tauri/tauri-serial.ts` / `web-serial.ts`. Larger
/// payloads from a runaway loop on the Ajisai side are rejected outright
/// instead of being committed in one IPC hop.
const MAX_WRITE_BYTES: usize = 1 << 20;

fn spawn_reader(
    app: AppHandle,
    port_id: String,
    mut reader: Box<dyn serialport::SerialPort>,
    stop: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    let _ = app.emit(
                        "serial-rx",
                        SerialRx {
                            port_id: port_id.clone(),
                            bytes: buf[..n].to_vec(),
                        },
                    );
                }
                // Read timeouts are the idle case for a polling reader.
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => {
                    let _ = app.emit(
                        "serial-disconnect",
                        SerialDisconnect {
                            port_id: port_id.clone(),
                        },
                    );
                    break;
                }
            }
        }
    });
}

#[tauri::command]
pub fn serial_list_ports() -> Result<Vec<String>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    Ok(ports.into_iter().map(|p| p.port_name).collect())
}

#[tauri::command]
pub fn serial_open(
    app: AppHandle,
    state: State<'_, SerialState>,
    port_id: String,
    baud_rate: u32,
) -> Result<(), String> {
    let mut ports = state.0.lock().map_err(|e| e.to_string())?;
    if ports.contains_key(&port_id) {
        return Ok(());
    }
    let port = serialport::new(&port_id, baud_rate)
        .timeout(READ_TIMEOUT)
        .open()
        .map_err(|e| e.to_string())?;
    let reader = port.try_clone().map_err(|e| e.to_string())?;
    let stop = Arc::new(AtomicBool::new(false));
    spawn_reader(app, port_id.clone(), reader, stop.clone());
    ports.insert(port_id, OpenPort { port, stop });
    Ok(())
}

#[tauri::command]
pub fn serial_configure(
    state: State<'_, SerialState>,
    port_id: String,
    baud_rate: u32,
) -> Result<(), String> {
    let mut ports = state.0.lock().map_err(|e| e.to_string())?;
    let entry = ports
        .get_mut(&port_id)
        .ok_or_else(|| format!("serial port '{port_id}' is not open"))?;
    entry.port.set_baud_rate(baud_rate).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn serial_write(
    state: State<'_, SerialState>,
    port_id: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if bytes.len() > MAX_WRITE_BYTES {
        return Err(format!(
            "serial write payload too large: {} > {}",
            bytes.len(),
            MAX_WRITE_BYTES
        ));
    }
    let mut ports = state.0.lock().map_err(|e| e.to_string())?;
    let entry = ports
        .get_mut(&port_id)
        .ok_or_else(|| format!("serial port '{port_id}' is not open"))?;
    entry.port.write_all(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn serial_flush(state: State<'_, SerialState>, port_id: String) -> Result<(), String> {
    let mut ports = state.0.lock().map_err(|e| e.to_string())?;
    let entry = ports
        .get_mut(&port_id)
        .ok_or_else(|| format!("serial port '{port_id}' is not open"))?;
    entry.port.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn serial_close(state: State<'_, SerialState>, port_id: String) -> Result<(), String> {
    let mut ports = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(entry) = ports.remove(&port_id) {
        // Signal the reader thread to exit; dropping `entry.port` closes the device.
        entry.stop.store(true, Ordering::Relaxed);
    }
    Ok(())
}
