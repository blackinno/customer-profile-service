use aws_config::{BehaviorVersion, Region};
use aws_sdk_sns::Client;

pub async fn initialize_aws_sns(region: String) -> Client {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .load()
        .await;

    Client::new(&config)
}
