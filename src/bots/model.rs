use crate::model::{BaseConfig, NatsConfig};
use log::info;
use nestify::nest;
use serde_derive::Deserialize;
use serde_yaml::Value;
use teloxide::prelude::Requester;
use teloxide::Bot as TBot;

nest! {
    #[derive(Default, Clone, Deserialize)]
    pub struct ConfigBots<'a> {
        logging: Option<String>,
        pub nats: NatsConfig<'a>,

        pub bot: Option<
            #[derive(Default, Clone, Deserialize)]
            pub struct BotConfig {
                pub tokens: Vec<String>,
            }>,

        pub args: Option<Value>,
    }
}

impl<'a> BaseConfig for ConfigBots<'a> {
    fn nats_config(&self) -> &NatsConfig<'a> {
        &self.nats
    }

    fn logging_config(&self) -> Option<String> {
        self.logging.clone()
    }
}

impl BotConfig {
    pub async fn get_bot(token: &str) -> TBot {
        let bot = TBot::new(token);
        let me = bot.get_me().await.expect("Failed to execute bot.get_me");

        info!("bot {} has started", me.username());

        bot
    }
    pub async fn get_bots(self) -> Vec<TBot> {
        let mut bots = Vec::new();

        for token in &self.tokens {
            let bot = Self::get_bot(token).await;
            bots.push(bot);
        }

        bots
    }
}
