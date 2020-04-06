use libp2p::gossipsub::Topic;
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GossipTopic {
    topic: String,
}

impl GossipTopic {
    pub fn new(topic: String) -> Self {
        GossipTopic { topic }
    }
}

impl Into<Topic> for GossipTopic {
    fn into(self) -> Topic {
        Topic::new(self.into())
    }
}

impl Into<String> for GossipTopic {
    fn into(self) -> String {
        self.topic
    }
}