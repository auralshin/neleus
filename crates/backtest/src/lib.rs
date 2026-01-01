use neleus_core_bus::{InMemoryBus, Message};
use neleus_core_engine::Engine;

pub struct BacktestNode {
    engine: Engine<InMemoryBus>,
}

impl BacktestNode {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(InMemoryBus::new()),
        }
    }

    pub fn start(&mut self) {
        self.engine.start();
    }

    pub fn stop(&mut self) {
        self.engine.stop();
    }

    pub fn publish(&mut self, message: Message) {
        self.engine.publish(message);
    }

    pub fn poll_once(&mut self) -> Option<Message> {
        self.engine.poll_once()
    }
}
