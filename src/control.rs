use std::{
    collections::VecDeque,
    net::UdpSocket,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use midir::{Ignore, MidiInput, MidiInputConnection};
use rosc::{OscMessage, OscPacket, OscType, decoder};

const MAX_CONTROL_EVENTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlEvent {
    Macro(usize, f32),
    Crossfader(f32),
    Blackout(bool),
    LiveMix(bool),
    Take(usize),
    PluginInput(usize, f32),
}

type EventQueue = Arc<Mutex<VecDeque<ControlEvent>>>;

pub struct ControlHub {
    queue: EventQueue,
    pub midi: MidiControl,
    pub osc: OscControl,
}

impl ControlHub {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        Self {
            midi: MidiControl::new(Arc::clone(&queue)),
            osc: OscControl::new(Arc::clone(&queue)),
            queue,
        }
    }

    pub fn drain(&self) -> Vec<ControlEvent> {
        self.queue
            .try_lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default()
    }
}

pub struct MidiControl {
    queue: EventQueue,
    pub ports: Vec<String>,
    pub selected: usize,
    pub connected: Option<String>,
    pub error: Option<String>,
    connection: Option<MidiInputConnection<()>>,
}

impl MidiControl {
    fn new(queue: EventQueue) -> Self {
        let mut control = Self {
            queue,
            ports: Vec::new(),
            selected: 0,
            connected: None,
            error: None,
            connection: None,
        };
        control.refresh();
        control
    }

    pub fn refresh(&mut self) {
        let result = MidiInput::new("TheVisualizer discovery").map(|input| {
            let ports = input.ports();
            ports
                .iter()
                .enumerate()
                .map(|(index, port)| {
                    input
                        .port_name(port)
                        .unwrap_or_else(|_| format!("MIDI input {}", index + 1))
                })
                .collect::<Vec<_>>()
        });
        match result {
            Ok(ports) => {
                self.ports = ports;
                self.selected = self.selected.min(self.ports.len().saturating_sub(1));
                self.error = None;
            }
            Err(error) => self.error = Some(format!("Could not enumerate MIDI inputs: {error}")),
        }
    }

    pub fn connect(&mut self) {
        let result: Result<(String, MidiInputConnection<()>), String> = (|| {
            let mut input =
                MidiInput::new("TheVisualizer MIDI").map_err(|error| error.to_string())?;
            input.ignore(Ignore::None);
            let ports = input.ports();
            let port = ports
                .get(self.selected)
                .ok_or_else(|| "Select an available MIDI input".to_owned())?
                .clone();
            let name = input
                .port_name(&port)
                .unwrap_or_else(|_| format!("MIDI input {}", self.selected + 1));
            let queue = Arc::clone(&self.queue);
            let connection = input
                .connect(
                    &port,
                    "TheVisualizer input",
                    move |_, message, _| {
                        if let Some(event) = midi_event(message) {
                            push_event(&queue, event);
                        }
                    },
                    (),
                )
                .map_err(|error| error.to_string())?;
            Ok((name, connection))
        })();
        match result {
            Ok((name, connection)) => {
                self.connection = Some(connection);
                self.connected = Some(name);
                self.error = None;
            }
            Err(error) => self.error = Some(format!("Could not connect MIDI: {error}")),
        }
    }

    pub fn disconnect(&mut self) {
        self.connection = None;
        self.connected = None;
    }
}

pub struct OscControl {
    queue: EventQueue,
    pub port: u16,
    pub running: bool,
    pub error: Option<String>,
    stop: Option<Arc<AtomicBool>>,
    thread: Option<JoinHandle<()>>,
}

impl OscControl {
    fn new(queue: EventQueue) -> Self {
        Self {
            queue,
            port: 9000,
            running: false,
            error: None,
            stop: None,
            thread: None,
        }
    }

    pub fn start(&mut self) {
        self.stop();
        let socket = match UdpSocket::bind(("127.0.0.1", self.port)) {
            Ok(socket) => socket,
            Err(error) => {
                self.error = Some(format!(
                    "Could not bind OSC 127.0.0.1:{}: {error}",
                    self.port
                ));
                return;
            }
        };
        if let Err(error) = socket.set_read_timeout(Some(Duration::from_millis(100))) {
            self.error = Some(format!("Could not configure OSC socket: {error}"));
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let queue = Arc::clone(&self.queue);
        self.thread = Some(thread::spawn(move || {
            let mut buffer = [0_u8; 65_535];
            while !worker_stop.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buffer) {
                    Ok((length, _)) => {
                        if let Ok((_, packet)) = decoder::decode_udp(&buffer[..length]) {
                            collect_osc(&queue, packet);
                        }
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(_) => break,
                }
            }
        }));
        self.stop = Some(stop);
        self.running = true;
        self.error = None;
    }

    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(worker) = self.thread.take() {
            let _ = worker.join();
        }
        self.running = false;
    }
}

impl Drop for OscControl {
    fn drop(&mut self) {
        self.stop();
    }
}

fn midi_event(message: &[u8]) -> Option<ControlEvent> {
    match message {
        [status, control, value, ..] if status & 0xf0 == 0xb0 => {
            let value = f32::from(*value) / 127.0;
            match *control {
                0..=9 => Some(ControlEvent::Macro(*control as usize, value)),
                10 => Some(ControlEvent::Crossfader(value)),
                11 => Some(ControlEvent::Blackout(value >= 0.5)),
                12 => Some(ControlEvent::LiveMix(value >= 0.5)),
                20..=27 => Some(ControlEvent::PluginInput((*control - 20) as usize, value)),
                _ => None,
            }
        }
        [status, note, velocity, ..] if status & 0xf0 == 0x90 && *velocity > 0 => match *note {
            36 => Some(ControlEvent::Take(0)),
            37 => Some(ControlEvent::Take(1)),
            _ => None,
        },
        _ => None,
    }
}

fn collect_osc(queue: &EventQueue, packet: OscPacket) {
    match packet {
        OscPacket::Message(message) => {
            if let Some(event) = osc_event(&message) {
                push_event(queue, event);
            }
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                collect_osc(queue, packet);
            }
        }
    }
}

fn osc_event(message: &OscMessage) -> Option<ControlEvent> {
    let value = message.args.first().and_then(osc_value);
    if let Some(index) = message
        .addr
        .strip_prefix("/thevisualizer/macro/")
        .and_then(|index| index.parse::<usize>().ok())
        .filter(|index| (1..=10).contains(index))
    {
        return Some(ControlEvent::Macro(index - 1, value?));
    }
    if let Some(index) = message
        .addr
        .strip_prefix("/thevisualizer/plugin/input/")
        .and_then(|index| index.parse::<usize>().ok())
        .filter(|index| (1..=8).contains(index))
    {
        return Some(ControlEvent::PluginInput(index - 1, value?));
    }
    match message.addr.as_str() {
        "/thevisualizer/crossfader" => Some(ControlEvent::Crossfader(value?)),
        "/thevisualizer/blackout" => Some(ControlEvent::Blackout(value? >= 0.5)),
        "/thevisualizer/live" => Some(ControlEvent::LiveMix(value? >= 0.5)),
        "/thevisualizer/take/a" => Some(ControlEvent::Take(0)),
        "/thevisualizer/take/b" => Some(ControlEvent::Take(1)),
        _ => None,
    }
}

fn osc_value(value: &OscType) -> Option<f32> {
    match value {
        OscType::Float(value) => Some(*value),
        OscType::Double(value) => Some(*value as f32),
        OscType::Int(value) => Some(*value as f32),
        OscType::Long(value) => Some(*value as f32),
        OscType::Bool(value) => Some(f32::from(*value)),
        _ => None,
    }
    .map(|value| value.clamp(0.0, 1.0))
}

fn push_event(queue: &EventQueue, event: ControlEvent) {
    if let Ok(mut queue) = queue.lock() {
        if queue.len() == MAX_CONTROL_EVENTS {
            queue.pop_front();
        }
        queue.push_back(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_and_osc_controls_map_to_bounded_events() {
        assert_eq!(
            midi_event(&[0xb0, 9, 127]),
            Some(ControlEvent::Macro(9, 1.0))
        );
        assert_eq!(midi_event(&[0x90, 36, 100]), Some(ControlEvent::Take(0)));
        assert_eq!(
            midi_event(&[0xb0, 20, 127]),
            Some(ControlEvent::PluginInput(0, 1.0))
        );
        assert_eq!(
            osc_event(&OscMessage {
                addr: "/thevisualizer/crossfader".to_owned(),
                args: vec![OscType::Float(1.4)],
            }),
            Some(ControlEvent::Crossfader(1.0))
        );
        assert_eq!(
            osc_event(&OscMessage {
                addr: "/thevisualizer/plugin/input/8".to_owned(),
                args: vec![OscType::Float(0.25)],
            }),
            Some(ControlEvent::PluginInput(7, 0.25))
        );
    }
}
