use crate::econ::model::LineState;
use futures_util::future::join_all;
use log::{debug, warn};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    #[default]
    Line,
    Random,
    All,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Task {
    Cron {
        cron: String,
        commands: Vec<String>,
        #[serde(default)]
        r#type: TaskType,
        #[serde(skip)]
        #[serde(default = "default_line_state")]
        state: LineState,
    },
    Delay {
        commands: Vec<String>,
        #[serde(default = "default_tasks_delay_sec")]
        delay: u64,
    },
}

impl Default for Task {
    fn default() -> Self {
        Task::Delay {
            delay: 5,
            commands: vec![String::new()],
        }
    }
}
impl Task {
    pub fn get_all_commands(&self) -> HashSet<String> {
        match self {
            Task::Cron { commands, .. } | Task::Delay { commands, .. } => {
                commands.iter().cloned().collect()
            }
        }
    }

    pub fn init_state(&mut self) {
        if let Task::Cron {
            commands, state, ..
        } = self
        {
            *state = LineState::new(commands.clone());
        }
    }

    pub async fn execute(&self, tx: &Sender<String>) {
        match self {
            Task::Cron {
                cron,
                state,
                r#type,
                ..
            } => {
                let schedule = match cron::Schedule::from_str(cron) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Invalid cron expression '{cron}': {e}");
                        return;
                    }
                };
                loop {
                    if let Some(next) = schedule.upcoming(chrono::Local).next() {
                        let duration = (next - chrono::Local::now())
                            .to_std()
                            .unwrap_or(Duration::ZERO);
                        sleep(duration).await;
                        Self::process_commands(tx, state, r#type).await;
                    }
                }
            }
            Task::Delay { delay, commands } => loop {
                for command in commands {
                    Self::send_command(tx, command).await;
                }
                sleep(Duration::from_secs(*delay)).await;
            },
        }
    }

    async fn process_commands(tx: &Sender<String>, state: &LineState, exec_type: &TaskType) {
        match exec_type {
            TaskType::Line => {
                let cmd = state.get_next_command();
                Self::send_command(tx, &cmd).await;
            }
            TaskType::Random => {
                let chosen = {
                    let mut rng = rand::thread_rng();
                    state.commands.choose(&mut rng).map(String::as_str)
                };

                match chosen {
                    None => warn!("random: No commands available"),
                    Some(result) => Self::send_command(tx, result).await,
                }
            }
            TaskType::All => {
                join_all(state.commands.iter().map(|cmd| Self::send_command(tx, cmd))).await;
            }
        }
    }

    async fn send_command(tx: &Sender<String>, command: &str) {
        debug!("tasks: send message to econ, msg: \"{command}\"");
        if let Err(e) = tx.send(command.to_string()).await {
            warn!("tx.send error, task failed: {e}");
        }
    }
}

fn default_line_state() -> LineState {
    LineState {
        index: Arc::new(AtomicUsize::new(0)),
        commands: Vec::new(),
    }
}

fn default_tasks_delay_sec() -> u64 {
    60
}
