/// Constants used throughout the CardiBot application

// Discord embed colors
pub const COLOR_SUCCESS: u32 = 0x238636; // Green

// API limits
pub const DISCORD_MESSAGE_FETCH_LIMIT: u8 = 50;
pub const GITHUB_THREAD_CONTENT_LIMIT: u8 = 10;

// Thread prefixes (fixed set for consistency)
pub const THREAD_PREFIXES: &[&str] = &["[BUG]", "[FEATURE]", "[QUESTION]", "[FEEDBACK]"];
pub const PREFIX_BUG: &str = "[BUG]";
pub const PREFIX_FEATURE: &str = "[FEATURE]";
pub const PREFIX_QUESTION: &str = "[QUESTION]";
pub const PREFIX_FEEDBACK: &str = "[FEEDBACK]";

// GitHub labels (as used in the API)
pub const LABEL_BUG: &str = "bug";
pub const LABEL_FEATURE: &str = "enhancement";  
pub const LABEL_QUESTION: &str = "question";
pub const LABEL_FEEDBACK: &str = "feedback";

// Bot messages
pub const MSG_ISSUE_CREATED: &str = "GitHub Issue Created";
pub const MSG_ISSUE_UPDATED: &str = "GitHub Issue Updated";
pub const MSG_ISSUE_CLOSED: &str = "🔒 Issue closed or merged on GitHub";
pub const MSG_ISSUE_REOPENED: &str = "🔓 Issue reopened on GitHub";

// Config defaults
pub const DEFAULT_CONFIG_PATH: &str = "config.toml";