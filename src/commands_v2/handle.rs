use std::sync::Arc;

use tokio::task::JoinSet;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;

use crate::commands_v2::builder::{CommandFunction, CommandGroup, CommandOption};
use crate::commands_v2::prelude::*;
use crate::commands_v2::request::Arg;
use crate::commands_v2::{bot, request};
use crate::utils::prelude::*;
use crate::{parser, Context};

pub async fn interaction_command(
    ctx: &Context,
    inter: Arc<Interaction>,
    data: Arc<CommandData>,
) -> CommandResult {
    // Lookup command from ctx.

    // TODO: Placeholder
    let base = Arc::new(
        bot::create_commands()
            .unwrap()
            .list
            .into_iter()
            .find(|c| c.command.name == data.name)
            .unwrap(),
    );

    // TODO: Restructure this for readability and DRY

    // Handle by kind.
    match data.kind {
        CommandType::ChatInput => {
            // Slash commands.

            // TODO: See if any twilight resolved data can be used as objects instead of ids.

            // Find last command.
            let cmd = base.command.to_owned();
            let mut last = Thing::Command(&cmd);
            let mut lookup_opts; // = base.command.options.to_vec();
            let mut data_opts = data.options.to_vec();
            let mut args = Vec::new();

            while let Some(opt) = data_opts.pop() {
                match opt.value {
                    CommandOptionValue::SubCommand(next)
                    | CommandOptionValue::SubCommandGroup(next) => {
                        lookup_opts = match last {
                            Thing::Command(c) => c.options.to_vec(),
                            Thing::Group(g) => g.to_options(),
                        };

                        // lookup option from lookup_opts
                        let found = lookup_opts
                            .iter()
                            .filter_map(Thing::from_option)
                            .find(|s| s.name() == opt.name);

                        match found {
                            Some(sub) => {
                                data_opts = next.to_vec(); // Set next option to check.
                                last = sub; // Set last command or group found.
                            },
                            None => {
                                // TODO: This should return error.
                                error!("Subcommand or group not found: {}", opt.name);
                                panic!("Subcommand or group not found: {}", opt.name);
                            },
                        }
                    },
                    arg => {
                        // Convert argument.
                        match arg.to_owned().try_into() {
                            Ok(arg) => {
                                args.push(request::Arg {
                                    name: opt.name,
                                    value: arg,
                                });
                            },
                            Err(e) => {
                                // TODO: This should return error.
                                error!("Could not process argument: '{}': {e}", arg.kind().kind());
                                panic!("Could not process argument: '{}': {e}", arg.kind().kind());
                            },
                        }
                    },
                }
            }

            let funcs = last
                .slash_functions()
                .context("Failed to get slash functions")?;

            // Task running.
            let mut set = JoinSet::new();
            let mut results = Vec::with_capacity(funcs.len());
            let req = SlashRequest::new(base, inter, data, args); // TODO: Should all the code above be in this function?

            for func in funcs {
                set.spawn(func.call(ctx.to_owned(), req.clone()));
            }

            // Wait for completion.
            while let Some(task) = set.join_next().await {
                let task = match task {
                    Ok(k) => k,
                    Err(e) => {
                        error!("{e}");
                        continue;
                    },
                };

                results.push(task);
            }

            // This should not fail.
            let last = results.pop().expect("No results from command handlers");

            for r in results {
                if let Err(e) = r {
                    error!("{e}");
                    // TODO: Collect all errors.
                    // Prioritize returning errors immediately, for now.
                    return Err(e);
                }
            }

            last
        },
        CommandType::User => {
            // User GUI commands.
            let data = data.resolved.as_ref().expect("Empty resolve error");
            println!("{data:#?}");

            // Globally.
            for _user in &data.users {}

            // Guilds only.
            for _member in &data.members {}

            // TODO: Create a modal for missing arguments? Custom modals per command? Create wrapper type instead?

            todo!();
        },
        CommandType::Message => {
            // Message GUI commands.
            let data = data.resolved.as_ref().expect("Empty resolve error");
            println!("{data:#?}");

            // Globally.
            for _message in &data.messages {}

            // TODO: Create a modal for missing arguments? Custom modals per command? Create wrapper type instead?
            todo!();
        },
        CommandType::Unknown(n) => panic!("Unknown command kind: {n}"),
    }

    // Ok(Response::Clear)
}

pub async fn classic_command(ctx: &Context, msg: Arc<Message>) -> CommandResult {
    let prefix = ctx.classic_prefix(msg.guild_id);
    let Some((_, unprefixed)) =  parser::unprefix_with([prefix], &msg.content) else {
        return Err(CommandError::NotPrefixed);
    };

    let (name, mut rest) = parser::split_once_whitespace(unprefixed);

    // Lookup command from ctx.
    // If no base command is found, all hope is lost.

    // TODO: Placeholder
    let base = Arc::new(
        bot::create_commands()
            .unwrap()
            .list
            .into_iter()
            .find(|c| c.command.name == name)
            .unwrap(),
    );

    // Find last command.
    let mut lookup = Thing::Command(&base.command);

    loop {
        let (name, next) = parser::split_once_whitespace(rest.unwrap_or(""));

        let found = match lookup {
            Thing::Command(f) => f
                .options
                .iter()
                .filter_map(Thing::from_option)
                .find(|t| t.name() == name),
            Thing::Group(g) => g.subs.iter().find(|s| s.name == name).map(Thing::Command),
        };

        if let Some(t) = found {
            lookup = t;
            rest = next;
            continue;
        }

        break;
    }

    let funcs = lookup
        .classic_functions()
        .context("Failed to get classic functions")?;

    // Task running.
    let mut set = JoinSet::new();
    let mut results = Vec::with_capacity(funcs.len());
    let req = ClassicRequest::new(Arc::clone(&base), msg);

    for func in funcs {
        set.spawn(func.call(ctx.to_owned(), req.clone()));
    }

    // Wait for completion.
    while let Some(task) = set.join_next().await {
        let task = match task {
            Ok(k) => k,
            Err(e) => {
                println!("{e}");
                error!("{e}");
                continue;
            },
        };

        results.push(task);
    }

    // This should not fail.
    let last = results.pop().expect("No results from command handlers");

    for r in results {
        if let Err(e) = r {
            error!("{e}");
            // TODO: Collect all errors.
            // Prioritize returning errors immediately, for now.
            return Err(e);
        }
    }

    last
}

enum Thing<'a> {
    Command(&'a CommandFunction),
    Group(&'a CommandGroup),
}

impl<'a> Thing<'a> {
    const fn from_option(opt: &'a CommandOption) -> Option<Self> {
        match opt {
            CommandOption::Sub(s) => Some(Self::Command(s)),
            CommandOption::Group(g) => Some(Self::Group(g)),
            _ => None,
        }
    }

    const fn name(&self) -> &str {
        match self {
            Thing::Command(t) => t.name,
            Thing::Group(t) => t.name,
        }
    }

    fn classic_functions(&self) -> AnyResult<Vec<Arc<dyn ClassicFunction>>> {
        match self {
            Thing::Command(c) if c.has_classic() => Ok(c
                .functions
                .iter()
                .filter_map(|f| match f {
                    Function::Classic(f) => Some(Arc::clone(f)),
                    _ => None,
                })
                .collect()),
            Thing::Command(c) => {
                anyhow::bail!("No classic commands found for command call: '{}'", c.name)
            },
            Thing::Group(_) => panic!("Oh no"), // TODO: This should be usage error or considered as an arg to previous command.
        }
    }

    fn slash_functions(&self) -> AnyResult<Vec<Arc<dyn SlashFunction>>> {
        match self {
            Thing::Command(c) if c.has_slash() => Ok(c
                .functions
                .iter()
                .filter_map(|f| match f {
                    Function::Slash(f) => Some(Arc::clone(f)),
                    _ => None,
                })
                .collect()),
            Thing::Command(c) => {
                anyhow::bail!("No slash commands found for command call: '{}'", c.name)
            },
            Thing::Group(g) => {
                anyhow::bail!("Expected a subcommand, found group: '{}'", g.name)
            },
        }
    }
}

pub struct CommandParser {
    args: Vec<Arg>,
}

impl CommandParser {}
