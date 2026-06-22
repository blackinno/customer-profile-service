use std::sync::Arc;

use crate::AwsSns;

#[derive(Clone)]
pub struct Message {
    pub sns: Arc<AwsSns>,
}
