use once_cell::sync::Lazy;
use crate::model::RegexModel;


pub static DD_PATTERNS: Lazy<Vec<RegexModel>> = Lazy::new(|| {
    vec![
        RegexModel::new("trainfngChatRegex", r"\[.*?]\[chat]: \d+:-?\d+:(.*): (.*)", None),
        RegexModel::new("trainfngJoinRegex", r"\[.*]\[.*]: \*\*\* '(.*)' (.*)", None),
        RegexModel::new("teeworldsChatRegex", r"\[chat]: \d+:-?\d+:(.*): (.*)", None),
        RegexModel::new("teeworldsLeaveRegex", r"\[game]: leave player='\d+:(.*)'", Some("{{text_leave}}")),
        RegexModel::new("teeworldsJoinRegex", r"\[game]: team_join player='\d+:(.*)' team=0", Some("{{text_join}}")),
        RegexModel::new("ddnetChatRegex", r".* I chat: \d+:-?\d+:(.*): (.*)", None),
        RegexModel::new("ddnetJoinRegex", r".* I chat: \*\*\* '(.*?)' (.*)", None),
    ]
});

pub static DD_PATTERNS_UTIL: Lazy<Vec<RegexModel>> = Lazy::new(|| {
    vec![
        RegexModel::new("rcon", r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} I server: ClientI[dD]=\d+ rcon='([^']+)'$", None),
        RegexModel::new("rcon", r"^\[server]: ClientID=\d+ rcon='([^']+)'$", None),
        RegexModel::new("rcon", r"\[.*?]\[server]: ClientID=.* rcon='(.*?)'", None),
    ]
});