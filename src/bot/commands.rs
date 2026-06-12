//! Built-in command definitions and handlers

use std::collections::HashMap;

pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    #[allow(dead_code)]
    pub usage: &'static str,
}

pub struct CommandResult {
    pub output: String,
}

pub struct CommandRegistry {
    commands: Vec<CommandInfo>,
    handlers: HashMap<&'static str, Box<dyn Fn(&str) -> CommandResult + Send + Sync>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            commands: Vec::new(),
            handlers: HashMap::new(),
        };
        reg.register_builtins();
        reg
    }

    fn register(&mut self, info: CommandInfo, handler: Box<dyn Fn(&str) -> CommandResult + Send + Sync>) {
        let name = info.name;
        self.commands.push(info);
        self.handlers.insert(name, handler);
    }

    fn register_builtins(&mut self) {
        self.register(
            CommandInfo { name: "help", description: "List all commands", usage: "/help" },
            Box::new(|_| {
                CommandResult { output: "Commands:\n/help — List commands\n/status — Pipeline status\n/translate on|off — Toggle translate\n/meeting start|stop — Toggle meeting notes\n/config — Open config panel\n/clear — Clear output\n/time — Current time\n/date — Current date\n/hello — Welcome message".into() }
            }),
        );

        self.register(
            CommandInfo { name: "status", description: "Show pipeline status", usage: "/status" },
            Box::new(|_| {
                CommandResult { output: "Use /translate on to start pipeline.\nUse /meeting start to begin meeting notes.".into() }
            }),
        );

        self.register(
            CommandInfo { name: "translate", description: "Toggle translate pipeline", usage: "/translate on|off" },
            Box::new(|args| {
                match args {
                    "on" => CommandResult { output: "Translate: ON".into() },
                    "off" => CommandResult { output: "Translate: OFF".into() },
                    _ => CommandResult { output: "Usage: /translate on|off".into() },
                }
            }),
        );

        self.register(
            CommandInfo { name: "meeting", description: "Toggle meeting notes", usage: "/meeting start|stop" },
            Box::new(|args| {
                match args {
                    "start" => CommandResult { output: "Meeting notes: STARTED".into() },
                    "stop" => CommandResult { output: "Meeting notes: STOPPED".into() },
                    _ => CommandResult { output: "Usage: /meeting start|stop".into() },
                }
            }),
        );

        self.register(
            CommandInfo { name: "config", description: "Open config panel", usage: "/config" },
            Box::new(|_| {
                CommandResult { output: "Opening config panel...".into() }
            }),
        );

        self.register(
            CommandInfo { name: "clear", description: "Clear dropdown output", usage: "/clear" },
            Box::new(|_| {
                CommandResult { output: String::new() }
            }),
        );

        self.register(
            CommandInfo { name: "time", description: "Show current time", usage: "/time" },
            Box::new(|_| {
                let now = chrono::Local::now();
                CommandResult { output: now.format("Time: %H:%M:%S").to_string() }
            }),
        );

        self.register(
            CommandInfo { name: "date", description: "Show current date", usage: "/date" },
            Box::new(|_| {
                let now = chrono::Local::now();
                CommandResult { output: now.format("Date: %A, %B %d, %Y").to_string() }
            }),
        );

        self.register(
            CommandInfo { name: "hello", description: "Welcome message", usage: "/hello" },
            Box::new(|_| {
                CommandResult { output: "Hello! I'm R Teams Bot. Type /help for available commands.".into() }
            }),
        );

        self.register(
            CommandInfo { name: "autoread", description: "Trigger auto-read cycle now", usage: "/autoread" },
            Box::new(|_| {
                CommandResult { output: "Auto-read cycle triggered".into() }
            }),
        );
    }

    pub fn commands(&self) -> &[CommandInfo] {
        &self.commands
    }

    pub fn execute(&self, command: &str, args: &str) -> CommandResult {
        match self.handlers.get(command) {
            Some(handler) => handler(args),
            None => CommandResult {
                output: format!("Unknown command: /{}. Type /help for available commands.", command),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_help() {
        let reg = CommandRegistry::new();
        let result = reg.execute("help", "");
        assert!(result.output.contains("/help"));
    }

    #[test]
    fn execute_unknown() {
        let reg = CommandRegistry::new();
        let result = reg.execute("foo", "");
        assert!(result.output.contains("Unknown command"));
    }

    #[test]
    fn execute_translate_on() {
        let reg = CommandRegistry::new();
        let result = reg.execute("translate", "on");
        assert!(result.output.contains("ON"));
    }

    #[test]
    fn execute_translate_invalid() {
        let reg = CommandRegistry::new();
        let result = reg.execute("translate", "maybe");
        assert!(result.output.contains("Usage"));
    }

    #[test]
    fn execute_time() {
        let reg = CommandRegistry::new();
        let result = reg.execute("time", "");
        assert!(result.output.contains("Time:"));
    }

    #[test]
    fn execute_date() {
        let reg = CommandRegistry::new();
        let result = reg.execute("date", "");
        assert!(result.output.contains("Date:"));
    }
}
