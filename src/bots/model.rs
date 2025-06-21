use crate::model::{BaseConfig, CowString};
use crate::nats::NatsConfig;
use log::info;
use nestify::nest;
use serde_derive::Deserialize;
use serde_yaml::Value;
use teloxide::prelude::Requester;
use teloxide::Bot as TBot;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct FormatConfig {
    pub format: CowString<'static>,
    #[serde(default)]
    pub escape: bool,
}

nest! {
    #[derive(Default, Debug, Clone, Deserialize)]
    pub struct ConfigBots<'a> {
        logging: Option<String>,
        pub nats: NatsConfig<'a>,

        pub bot:
            #[derive(Default, Debug, Clone, Deserialize)]
            pub struct BotConfig {
                pub tokens: Vec<String>,
            },

        #[serde(default = "default_format")]
        pub format:
            #[derive(Default, Debug, Clone, Deserialize)]
            pub struct FormatsConfigs {
                #[serde(default = "default_formats_text")]
                pub text: Vec<FormatConfig>,
                #[serde(default = "default_formats_reply")]
                pub reply: Vec<FormatConfig>,
                #[serde(default = "default_formats_media")]
                pub media: String,
                #[serde(default = "default_formats_sticker")]
                pub sticker: String,
            },

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

fn default_format() -> FormatsConfigs {
    FormatsConfigs {
        text: default_formats_text(),
        reply: default_formats_reply(),
        media: default_formats_media(),
        sticker: default_formats_sticker(),
    }
}

fn default_formats_text() -> Vec<FormatConfig> {
    vec![
        FormatConfig {
            format: CowString::Owned(String::from("{{2}}[{{from.username}}] {{0}}")),
            escape: true,
        },
        FormatConfig {
            format: CowString::Owned(String::from("say \"{{1}}\"")),
            escape: false,
        },
    ]
}

fn default_formats_reply() -> Vec<FormatConfig> {
    vec![
        FormatConfig {
            format: CowString::Owned(String::from(
                "{{2}}[{{reply_to_message.message_id}}] [{{reply_to_message.from.username}}] {{0}}",
            )),
            escape: true,
        },
        FormatConfig {
            format: CowString::Owned(String::from("say \"{{1}}\"")),
            escape: false,
        },
    ]
}

fn default_formats_media() -> String {
    String::from("[MEDIA] ")
}

fn default_formats_sticker() -> String {
    String::from("[STICKER] ")
}
