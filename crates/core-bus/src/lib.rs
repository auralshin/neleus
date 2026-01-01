use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKind {
    Data,
    Event,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub kind: MessageKind,
    pub topic: String,
    pub payload: Vec<u8>,
}

pub trait Bus {
    fn publish(&mut self, message: Message);
    fn poll(&mut self) -> Option<Message>;
    fn len(&self) -> usize;
}

pub struct InMemoryBus {
    queue: VecDeque<Message>,
}

impl InMemoryBus {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl Bus for InMemoryBus {
    fn publish(&mut self, message: Message) {
        self.queue.push_back(message);
    }

    fn poll(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}
