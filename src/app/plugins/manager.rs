use std::collections::HashSet;
use std::path::PathBuf;
use std::thread;

use winit::event_loop::EventLoopProxy;

use yatmux::config::Config;

use super::discovery::discover_plugins;
use super::events::PluginEvent;
use super::runtime::run_plugin;
use super::utils::should_deliver_event;
use crate::app::AppEvent;

#[derive(Debug, Clone)]
pub struct PluginManager {
    plugins: Vec<Plugin>,
    config_path: Option<PathBuf>,
    subscriptions: std::collections::HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct Plugin {
    pub(super) name: String,
    pub(super) root: PathBuf,
    pub(super) script: PathBuf,
}

impl PluginManager {
    pub fn new(config: &Config) -> Self {
        let plugins = if config.plugins.enabled {
            discover_plugins(config, &config.plugins)
        } else {
            Vec::new()
        };
        let subscriptions = plugins
            .iter()
            .map(|plugin| (plugin.name.clone(), HashSet::new()))
            .collect();
        Self {
            plugins,
            config_path: Config::config_path(),
            subscriptions,
        }
    }

    pub fn reload(&mut self, config: &Config) {
        *self = Self::new(config);
    }

    pub fn dispatch(
        &self,
        event: PluginEvent,
        proxy: Option<EventLoopProxy<AppEvent>>,
        target_plugin: Option<&str>,
    ) {
        let Some(proxy) = proxy else {
            return;
        };
        if self.plugins.is_empty() {
            return;
        }
        let plugins = self.plugins.clone();
        let config_path = self.config_path.clone();
        let subscriptions = self.subscriptions.clone();
        let target_plugin = target_plugin.map(|s| s.to_string());
        thread::spawn(move || {
            let payload = match serde_json::to_string(&event) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Plugin event serialize failed: {e}");
                    return;
                }
            };

            for plugin in plugins {
                if let Some(target) = target_plugin.as_deref() {
                    if plugin.name != target {
                        continue;
                    }
                } else if !should_deliver_event(&subscriptions, &plugin.name, &event.event) {
                    continue;
                }
                if let Some(commands) = run_plugin(&plugin, &payload, config_path.as_deref()) {
                    if !commands.is_empty() {
                        let _ = proxy.send_event(AppEvent::PluginCommands {
                            plugin: plugin.name.clone(),
                            commands,
                        });
                    }
                }
            }
        });
    }

    pub fn set_subscription(&mut self, plugin: &str, events: Vec<String>) {
        let set: HashSet<String> = events
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.subscriptions.insert(plugin.to_string(), set);
    }
}
