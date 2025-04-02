use nestify::nest;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::option::Option;

nest! {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct MsgHandlerAuto {
        pub data:
            #[derive(Clone, Debug, Serialize, Deserialize)]
            pub struct Data {
                pub timestamp: Option<i64>,
                pub text: String,
                pub logging_level: String,
                pub logging_name: String,
            },
        pub args: Value
    }
}
