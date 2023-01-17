use std::sync::Arc;

use tokio::task::JoinSet;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Message;

use crate::commands_v2::builder::{CommandFunction, CommandGroup, CommandOption};
use crate::commands_v2::function::{Callable, ClassicFunction, SlashFunction};
use crate::commands_v2::prelude::*;
use crate::commands_v2::request::Arg;
use crate::commands_v2::{bot, request};
use crate::utils::prelude::*;
use crate::{parser, Context};

/// Handle interaction and execute command functions.
pub async fn interaction_command(
    ctx: &Context,
    inter: Arc<Interaction>,
    data: Arc<CommandData>,
) -> CommandResult {
    // Lookup command from context.
    let Some(base) = ctx.commands.get(data.name.as_str()) else {
        return Err(CommandError::NotFound(format!("Command '{}' does not exist", data.name)))
    };

    let base = Arc::new(base.to_owned());

    // TODO: Restructure this for readability

    // Handle by kind.
    match data.kind {
        CommandType::ChatInput => {
            // Slash commands.

            // TODO: See if any twilight resolved data can be used as objects instead of ids.

            // Find last command.
            let cmd = base.command.to_owned();
            let mut last = Lookup::Command(&cmd);
            let mut lookup_opts; // = base.command.options.to_vec();
            let mut data_opts = data.options.to_vec();
            let mut args = Vec::new();

            while let Some(opt) = data_opts.pop() {
                match opt.value {
                    CommandOptionValue::SubCommand(next)
                    | CommandOptionValue::SubCommandGroup(next) => {
                        lookup_opts = match last {
                            Lookup::Command(c) => c.options.to_vec(),
                            Lookup::Group(g) => g.to_options(),
                        };

                        // lookup option from lookup_opts
                        let found = lookup_opts
                            .iter()
                            .filter_map(Lookup::from_option)
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

            let req = SlashRequest::new(base, inter, data, args); // TODO: Should all the code above be in this function?

            execute(ctx, funcs, req).await
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

/// Parse message and execute command functions.
pub async fn classic_command(ctx: &Context, msg: Arc<Message>) -> CommandResult {
    // Unprefix the message contents.
    let prefix = ctx.classic_prefix(msg.guild_id);
    let Some((_, unprefixed)) =  parser::unprefix_with([prefix], &msg.content) else {
        return Err(CommandError::NotPrefixed);
    };

    // Get first possible command name.
    let (name, mut rest) = parser::split_once_whitespace(unprefixed);

    // Lookup command from context.
    let Some(base) = ctx.commands.get(name) else {
        return Err(CommandError::NotFound(format!("Command '{name}' does not exist")))
    };

    let base = Arc::new(base.to_owned());
    let mut lookup = Lookup::Command(&base.command);

    // Parse contents until last (sub)command is found.
    loop {
        let (name, next) = parser::split_once_whitespace(rest.unwrap_or(""));

        let found = match lookup {
            Lookup::Command(f) => f
                .options
                .iter()
                .filter_map(Lookup::from_option)
                .find(|t| t.name() == name),
            Lookup::Group(g) => g.subs.iter().find(|s| s.name == name).map(Lookup::Command),
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

    let req = ClassicRequest::new(Arc::clone(&base), msg);

    execute(ctx, funcs, req).await
}

enum Lookup<'a> {
    Command(&'a CommandFunction),
    Group(&'a CommandGroup),
}

impl<'a> Lookup<'a> {
    const fn from_option(opt: &'a CommandOption) -> Option<Self> {
        match opt {
            CommandOption::Sub(s) => Some(Self::Command(s)),
            CommandOption::Group(g) => Some(Self::Group(g)),
            _ => None,
        }
    }

    const fn name(&self) -> &str {
        match self {
            Lookup::Command(t) => t.name,
            Lookup::Group(t) => t.name,
        }
    }

    fn classic_functions(&self) -> AnyResult<Vec<Arc<dyn ClassicFunction>>> {
        match self {
            Lookup::Command(c) if c.has_classic() => Ok(c
                .functions
                .iter()
                .filter_map(|f| match f {
                    Function::Classic(f) => Some(Arc::clone(f)),
                    _ => None,
                })
                .collect()),
            Lookup::Command(c) => {
                anyhow::bail!("No classic commands found for command call: '{}'", c.name)
            },
            Lookup::Group(_) => panic!("Oh no"), // TODO: This should be usage error or considered as an arg to previous command.
        }
    }

    fn slash_functions(&self) -> AnyResult<Vec<Arc<dyn SlashFunction>>> {
        match self {
            Lookup::Command(c) if c.has_slash() => Ok(c
                .functions
                .iter()
                .filter_map(|f| match f {
                    Function::Slash(f) => Some(Arc::clone(f)),
                    _ => None,
                })
                .collect()),
            Lookup::Command(c) => {
                anyhow::bail!("No slash commands found for command call: '{}'", c.name)
            },
            Lookup::Group(g) => {
                anyhow::bail!("Expected a subcommand, found group: '{}'", g.name)
            },
        }
    }
}

pub struct CommandParser {
    args: Vec<Arg>,
}

impl CommandParser {} // TODO

/// Execute tasks.
async fn execute<F, R>(ctx: &Context, funcs: Vec<F>, req: R) -> CommandResult
where
    F: Callable<R>,
    R: Clone,
{
    let mut set = JoinSet::new();
    let mut results = Vec::with_capacity(funcs.len());

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
