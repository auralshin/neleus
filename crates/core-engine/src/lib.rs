use neleus_core_bus::{Bus, Message};

pub struct Engine<B: Bus> {
    bus: B,
    is_running: bool,
}

impl<B: Bus> Engine<B> {
    pub fn new(bus: B) -> Self {
        Self {
            bus,
            is_running: false,
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn poll_once(&mut self) -> Option<Message> {
        if !self.is_running {
            return None;
        }
        self.bus.poll()
    }

    pub fn publish(&mut self, message: Message) {
        self.bus.publish(message);
    }
}
