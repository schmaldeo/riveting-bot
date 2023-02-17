use std::sync::Arc;

use tokio::task::JoinSet;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::Message;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

use crate::commands_v2::arg::{Arg, Args};
use crate::commands_v2::builder::{
    ArgDesc, BaseCommand, CommandFunction, CommandGroup, CommandOption,
};
use crate::commands_v2::function::{Callable, ClassicFunction, SlashFunction};
use crate::commands_v2::prelude::*;
use crate::utils::prelude::*;
use crate::{parser, Context};

const ERROR_MESSAGE: &str = "The bot has encountered an error executing the command! :confused:";

/// Handle interaction and execute command functions.
pub async fn application_command(
    ctx: &Context,
    inter: Interaction,
    data: CommandData,
) -> Result<(), CommandError> {
    // Lookup command from context.
    let Some(base) = ctx.commands.get(data.name.as_str()) else {
        return Err(CommandError::NotFound(format!("Command '{}' does not exist", data.name)))
    };

    let base = Arc::clone(base);
    let inter = Arc::new(inter);
    let data = Arc::new(data);

    // Process the command by kind.
    let result = {
        let inter = Arc::clone(&inter);
        match data.kind {
            CommandType::ChatInput => process_slash(ctx, base, inter, data).await,
            CommandType::Message => process_message(ctx, base, inter, data).await,
            CommandType::User => process_user(ctx, base, inter, data).await,
            CommandType::Unknown(n) => panic!("Unknown command kind: {n}"),
            other => panic!("Unhandled command kind: {other:?}"),
        }
    };

    let interaction = ctx.interaction();

    // Handle execution result.
    // Catch erroneous execution and clear dangling response.
    match result {
        Ok(Response::None | Response::Clear) => {
            // Clear deferred message response.
            interaction
                .delete_response(&inter.token)
                .await
                .context("Failed to clear interaction")?;
        },
        Ok(Response::CreateMessage(text)) => {
            interaction
                .update_response(&inter.token)
                .content(Some(&text))
                .context("Response message error")?
                .await
                .context("Failed to send response message")?;
        },
        Err(e) => {
            interaction
                .create_followup(&inter.token)
                .flags(MessageFlags::EPHEMERAL)
                .content(ERROR_MESSAGE)?
                .await
                .context("Failed to send error message")?;

            return Err(e);
        },
    }

    Ok(())
}

/// Slash interaction commands.
async fn process_slash(
    ctx: &Context,
    base: Arc<BaseCommand>,
    inter: Arc<Interaction>,
    data: Arc<CommandData>,
) -> CommandResult {
    // Acknowledge the interaction.
    normal_acknowledge(ctx, &inter).await?;

    let mut args = Vec::new();
    let mut last = Lookup::Command(&base.command);
    let mut data_opts = data.options.to_vec();
    let mut lookup_opts; // Declared here for lifetime reasons.

    // Process interaction until last (sub)command is found.
    // This processes options in reverse, it is fine however,
    // because `CommandDataOption` is a nested structure and
    // only holds one type of options in the `value` field.
    while let Some(opt) = data_opts.pop() {
        match opt.value {
            CommandOptionValue::SubCommand(next) | CommandOptionValue::SubCommandGroup(next) => {
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
                        // Args are still stored in reverse order.
                        args.push(Arg {
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

    // Reverse the args to the correct order for arbitrary reasons.
    args.reverse();

    let funcs = last
        .slash_functions()
        .context("Failed to get slash functions")?;

    let req = SlashRequest::new(
        Arc::clone(&base),
        Arc::clone(&inter),
        data,
        Args::from(args),
    );

    execute(ctx, funcs, req).await
}

// TODO: See if any twilight resolved data can be used as objects instead of ids.
/// Message GUI interaction commands.
async fn process_message(
    ctx: &Context,
    base: Arc<BaseCommand>,
    inter: Arc<Interaction>,
    data: Arc<CommandData>,
) -> CommandResult {
    // Acknowledge the interaction.
    ephemeral_acknowledge(ctx, &inter).await?;

    // let data = data.resolved.as_ref().expect("Empty resolve error");
    // for _message in &data.messages {} // Globally.

    let target = data.target_id.ok_or(CommandError::MissingArgs)?.cast();
    let req = MessageRequest::new(Arc::clone(&base), inter, data, target);
    execute(ctx, base.command.message(), req).await
}

// TODO: See if any twilight resolved data can be used as objects instead of ids.
/// User GUI interaction commands.
async fn process_user(
    ctx: &Context,
    base: Arc<BaseCommand>,
    inter: Arc<Interaction>,
    data: Arc<CommandData>,
) -> CommandResult {
    // Acknowledge the interaction.
    ephemeral_acknowledge(ctx, &inter).await?;

    // let data = data.resolved.as_ref().expect("Empty resolve error");
    // for _user in &data.users {} // Globally.
    // for _member in &data.members {} // Guilds only.

    let target = data.target_id.ok_or(CommandError::MissingArgs)?.cast();
    let req = UserRequest::new(Arc::clone(&base), inter, data, target);
    execute(ctx, base.command.user(), req).await
}

/// Creates a publicly visible loading state message.
async fn normal_acknowledge(ctx: &Context, inter: &Interaction) -> AnyResult<()> {
    let interaction = ctx.interaction();

    let resp = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    };

    interaction
        .create_response(inter.id, &inter.token, &resp)
        .await?;

    Ok(())
}

/// Creates a personal loading state message.
async fn ephemeral_acknowledge(ctx: &Context, inter: &Interaction) -> AnyResult<()> {
    let interaction = ctx.interaction();

    let resp = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: Some(InteractionResponseData {
            flags: Some(MessageFlags::EPHEMERAL | MessageFlags::LOADING),
            ..Default::default()
        }),
    };

    interaction
        .create_response(inter.id, &inter.token, &resp)
        .await?;

    Ok(())
}

/// Parse message and execute command functions.
pub async fn classic_command(ctx: &Context, msg: Arc<Message>) -> Result<(), CommandError> {
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

    let args = match lookup {
        Lookup::Command(c) => parse_classic_args(c, &msg, rest)?,
        Lookup::Group(g) => {
            return Err(CommandError::UnexpectedArgs(format!(
                "Expected command, found group '{}'",
                g.name
            )));
        },
    };

    let funcs = lookup
        .classic_functions()
        .context("Failed to get classic functions")?;

    trace!(
        "Creating classic request for '{name}' by user '{}'",
        msg.author.id
    );

    let req = ClassicRequest::new(Arc::clone(&base), Arc::clone(&msg), args);

    debug!("Executing '{name}' by user '{}'", msg.author.id);

    let response = execute(ctx, funcs, req).await;

    trace!("Completing '{name}' by user '{}'", msg.author.id);

    // Handle execution result.
    match response {
        Ok(Response::None) => (),
        Ok(Response::Clear) => {
            ctx.http
                .delete_message(msg.channel_id, msg.id)
                .await
                .context("Failed to clear command message")?;
        },
        Ok(Response::CreateMessage(text)) => {
            ctx.http
                .create_message(msg.channel_id)
                .reply(msg.id)
                .content(&format!("{text}\n"))
                .context("Response message error")?
                .await
                .context("Failed to send response message")?;
        },
        Err(e) => {
            ctx.http
                .create_message(msg.channel_id)
                .reply(msg.id)
                .content(ERROR_MESSAGE)?
                .await?;

            return Err(e);
        },
    }

    Ok(())
}

fn parse_classic_args(
    cmd_fn: &CommandFunction,
    msg: &Message,
    mut rest: Option<&str>,
) -> Result<Args, CommandError> {
    fn parse(arg: &ArgDesc, msg: &Message, rest: &mut Option<&str>) -> AnyResult<Arg> {
        // Normal arguments parsing.
        let normal = || {
            let unparsed = rest.ok_or(CommandError::MissingArgs)?;
            let (value, next) = parser::maybe_quoted_arg(unparsed).with_context(|| {
                format!("Failed to parse next argument from content '{unparsed}'")
            })?;
            *rest = next;

            Arg::from_desc(arg, value).with_context(|| {
                format!("Expected an argument '{}' of type '{}'", arg.name, arg.kind)
            })
        };

        // Handle special arguments.
        Arg::from_desc_msg(arg, msg)
            .map(|special| special.map_or_else(normal, Ok)) // If special returned `None`, try normal parsing.
            .with_context(|| {
                format!("Expected an argument '{}' of type '{}'", arg.name, arg.kind)
            })?
    }

    let mut parsed = Vec::new();
    let args: Vec<_> = cmd_fn.args().collect();
    let mut split = args.iter().position(|a| !a.required).unwrap_or(args.len());

    // Process all the required args.
    for arg in &args[..split] {
        let arg = parse(arg, msg, &mut rest).context("Required argument error")?;
        parsed.push(arg);
    }

    // TODO: This still assumes lock-step arg ordering in input, when it should not.
    // Process rest of the args, if any.
    while rest.is_some() && split < args.len() {
        let arg = args[split];
        let arg = parse(arg, msg, &mut rest).context("Optional argument error")?;
        parsed.push(arg);
        split += 1;
    }

    Ok(Args::from(parsed))
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

    fn classic_functions(&self) -> AnyResult<impl Iterator<Item = ClassicFunction> + '_> {
        match self {
            Lookup::Command(c) if c.has_classic() => Ok(c.classic()),
            Lookup::Command(c) => {
                anyhow::bail!("No classic commands found for command call: '{}'", c.name)
            },
            Lookup::Group(g) => {
                // TODO: This should be usage error or considered as an arg to previous command.
                anyhow::bail!("Expected a subcommand, found group: '{}'", g.name)
            },
        }
    }

    fn slash_functions(&self) -> AnyResult<impl Iterator<Item = SlashFunction> + '_> {
        match self {
            Lookup::Command(c) if c.has_slash() => Ok(c.slash()),
            Lookup::Command(c) => {
                anyhow::bail!("No slash commands found for command call: '{}'", c.name)
            },
            Lookup::Group(g) => {
                anyhow::bail!("Expected a subcommand, found group: '{}'", g.name)
            },
        }
    }
}

/// Execute tasks.
async fn execute<F, R>(ctx: &Context, funcs: impl Iterator<Item = F>, req: R) -> CommandResult
where
    F: Callable<R>,
    R: Clone,
{
    let mut set = JoinSet::new();
    let mut results = Vec::new();

    for func in funcs {
        set.spawn(func.call(ctx.to_owned(), req.clone()));
    }

    // Wait for completion.
    while let Some(task) = set.join_next().await {
        results.push(task.context("Execution task join error"));
    }

    // This should not fail.
    let last = results.pop().expect("No results from command handlers");

    for r in results {
        // TODO: Collect all errors and responses.
        // Prioritize returning errors immediately, for now.
        r?.ok();
    }

    last?.context("Execution result").map_err(Into::into)
}
