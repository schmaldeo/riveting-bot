use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time;
use twilight_gateway::Event;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::select_menu::SelectMenuOption;
use twilight_model::application::component::{ActionRow, Button, Component, SelectMenu};
use twilight_model::application::interaction::MessageComponentInteraction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::{Message, Reaction, ReactionType};
use twilight_model::gateway::payload::incoming::RoleUpdate;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::{GuildMarker, RoleMarker};
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils;
use crate::utils::prelude::*;

/// Command: Manage reaction-roles.
pub async fn roles(cc: CommandContext<'_>) -> CommandResult {
    if cc.msg.guild_id.is_none() {
        return Err(CommandError::Disabled);
    }

    cc.http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&format!("```{}```", cc.cmd))?
        .send()
        .await?;

    Ok(())
}

/// Command: Setup a reaction-roles message.
pub async fn setup(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let Some(mappings) = roles_setup_process(&cc, guild_id, None).await? else {
        return Ok(()) // Canceled or whatever.
    };

    let list = display_emoji_roles(&cc, guild_id, &mappings).await?;
    let output_content = indoc::formatdoc! {"
        React to give yourself some roles:

        {list}
        "
    };

    let output = cc
        .http
        .create_message(cc.msg.channel_id)
        .content(&output_content)?
        .send()
        .await?;

    add_reactions_to_message(&cc, &mappings, &output).await?;

    let mut lock = cc.config.lock().unwrap();
    lock.add_reaction_roles(guild_id, output.channel_id, output.id, mappings);
    lock.write_guild(guild_id)?;

    Ok(())
}

/// Command: Edit a reaction-roles mapping.
pub async fn edit(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let Some(replied) = &cc.msg.referenced_message else {
        return Err(CommandError::MissingReply);
    };

    // Ignore if replied message is not from this bot.
    if replied.author.id != cc.user.id {
        return Err(CommandError::UnexpectedArgs(
            "Replied message is not from this bot".to_string(),
        ));
    }

    let reaction_roles = {
        let lock = cc.config.lock().unwrap();
        lock.guild(guild_id)
            .and_then(|s| {
                let key = format!("{}.{}", replied.channel_id, replied.id);
                s.reaction_roles.get(&key)
            })
            .cloned()
    };

    if reaction_roles.is_none() {
        return Err(CommandError::UnexpectedArgs(
            "Message is not a reaction-roles post".to_string(),
        ));
    }

    let Some(mappings) = roles_setup_process(&cc, guild_id, reaction_roles).await? else {
        return Ok(()) // Canceled or whatever.
    };

    let list = display_emoji_roles(&cc, guild_id, &mappings).await?;
    let output_content = indoc::formatdoc! {"
        React to give yourself some roles:

        {list}
        "
    };

    // NOTE This will just overwrite all content of the original message.
    let output = cc
        .http
        .update_message(replied.channel_id, replied.id)
        .content(Some(&output_content))?
        .send()
        .await?;

    add_reactions_to_message(&cc, &mappings, &output).await?;

    let mut lock = cc.config.lock().unwrap();
    lock.add_reaction_roles(guild_id, output.channel_id, output.id, mappings);
    lock.write_guild(guild_id)?;

    Ok(())
}

async fn roles_setup_process(
    cc: &CommandContext<'_>,
    guild_id: Id<GuildMarker>,
    preset: Option<Vec<ReactionRole>>,
) -> Result<Option<Vec<ReactionRole>>, CommandError> {
    let info_text = indoc::formatdoc! {"
    **Reaction-roles setup**
    
        *Step 1:*  React to this message.
        *Step 2:*  Select a role from the dropdown menu.
        *Step 3:*  Repeat until you are happy with the role mappings.
        *Step 4:*  Press **Done** to finalize the reaction-roles message.
    
    If any role is not displayed in the list, it may be too stronk for the bot.
    "};

    let author_id = cc.msg.author.id;
    let interaction = cc.http.interaction(cc.application.id);
    let mut mappings = preset.unwrap_or_default();

    // Initial content of the controller message.
    let content = {
        let list = display_emoji_roles(cc, guild_id, &mappings).await?;
        format!("{info_text}\n{list}")
    };

    // Setup message with controls.
    let mut controller = cc
        .http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content(&content)?
        .components(&controller_components(true))?
        .send()
        .await?;

    // Add any previous reactions if this is an edit.
    add_reactions_to_message(cc, &mappings, &controller).await?;

    let controller_mci = loop {
        // Future that waits for controller button press.
        let controller_fut = cc
            .standby
            .wait_for_component(controller.id, move |event: &MessageComponentInteraction| {
                event.author_id() == Some(author_id)
            });

        // Future that waits for a reaction add or remove.
        let reaction_fut = {
            let id = controller.id;
            let channel_id = controller.channel_id;

            cc.standby
                .wait_for(guild_id, move |event: &Event| match event {
                    Event::ReactionAdd(r) => {
                        r.message_id == id && r.channel_id == channel_id && r.user_id == author_id
                    },
                    Event::ReactionRemove(r) => {
                        r.message_id == id && r.channel_id == channel_id && r.user_id == author_id
                    },
                    _ => false,
                })
        };

        // Wait for a reaction or a controller button.
        let event = tokio::select! {
            biased;
            event = reaction_fut => event?, // Proceed with the reaction event.
            mci = controller_fut => break mci?, // Exit loop with button interaction.
        };

        match event {
            Event::ReactionAdd(added) => {
                // Show roles dropdown list and add a role mapping.

                // If already mapped, ignore it.
                if mappings
                    .iter()
                    .any(|ReactionRole { emoji, .. }| utils::reaction_type_eq(emoji, &added.emoji))
                {
                    continue;
                }

                let components = dropdown_components(cc, guild_id, &added).await?;

                // Gray out controller buttons.
                update_controller(cc, &mut controller, None, false).await?;

                // Create dropdown list interaction.
                let dropdown = cc
                    .http
                    .create_message(cc.msg.channel_id)
                    .reply(cc.msg.id)
                    .components(&components)?
                    .send()
                    .await?;

                // Wait for user to select an option.
                let mut list_mci = cc
                    .standby
                    .wait_for_component(dropdown.id, move |event: &MessageComponentInteraction| {
                        event.author_id() == Some(author_id)
                    })
                    .await?;

                let resp = InteractionResponse {
                    kind: InteractionResponseType::DeferredUpdateMessage,
                    data: Some(InteractionResponseData::default()),
                };

                // Acknowledge the interaction.
                interaction
                    .create_response(list_mci.id, &list_mci.token, &resp)
                    .exec()
                    .await?;

                // Delete the dropdown message.
                interaction.delete_response(&list_mci.token).exec().await?;

                let choice = list_mci.data.values.pop().unwrap_or_default();

                if choice == "cancel" {
                    // Canceling, re-enable controller buttons.
                    update_controller(cc, &mut controller, None, true).await?;

                    // Remove canceled reaction.
                    let request_emoji = request_from_emoji(&added.emoji);

                    cc.http
                        .delete_all_reaction(controller.channel_id, controller.id, &request_emoji)
                        .exec()
                        .await?;

                    continue;
                }

                match choice.parse::<Id<RoleMarker>>() {
                    Ok(role_id) => {
                        // Save the choice.
                        mappings.push(ReactionRole::new(added.emoji.to_owned(), role_id));

                        // Create a message that lists all roles that have been added so far.
                        let list = display_emoji_roles(cc, guild_id, &mappings).await?;
                        let content = format!("{info_text}\n{list}");

                        // Update the controller message and re-enable controller buttons.
                        update_controller(cc, &mut controller, Some(&content), true).await?;
                    },
                    Err(e) => {
                        // Error parsing the choice.
                        warn!("Could not parse role choice: {e}");

                        // Update the controller message and re-enable controller buttons.
                        update_controller(cc, &mut controller, None, true).await?;
                    },
                }
            },
            Event::ReactionRemove(removed) => {
                // Remove any mapping that has this emoji.
                let _ = mappings
                    .iter()
                    .enumerate()
                    .find(|(_, r)| utils::reaction_type_eq(&r.emoji, &removed.emoji))
                    .map(|(idx, _)| idx)
                    .map(|idx| mappings.remove(idx));

                let list = display_emoji_roles(cc, guild_id, &mappings).await?;
                let content = format!("{info_text}\n{list}");

                update_controller(cc, &mut controller, Some(&content), true).await?;

                let request_emoji = request_from_emoji(&removed.emoji);

                cc.http
                    .delete_all_reaction(controller.channel_id, controller.id, &request_emoji)
                    .exec()
                    .await?;
            },
            _ => {
                warn!("Unexpected event type in reaction-roles setup mapping");
                continue;
            },
        }
    };

    if controller_mci.data.custom_id == "roles_cancel" {
        // Delete the controller message.
        cc.http
            .delete_message(controller.channel_id, controller.id)
            .exec()
            .await?;

        // Delete the original command message.
        cc.http
            .delete_message(cc.msg.channel_id, cc.msg.id)
            .exec()
            .await?;

        // Nothing more to be done here.
        return Ok(None);
    }

    // If no reaction-roles were added or all were removed.
    if mappings.is_empty() {
        let text = "Well, that was kinda pointless... This message will self-destruct in 5 \
                    seconds."
            .to_string();

        let resp = InteractionResponse {
            kind: InteractionResponseType::UpdateMessage,
            data: Some(InteractionResponseDataBuilder::new().content(text).build()),
        };

        interaction
            .create_response(controller_mci.id, &controller_mci.token, &resp)
            .exec()
            .await?;

        time::sleep(Duration::from_secs(5)).await;

        // Delete the controller message.
        cc.http
            .delete_message(controller.channel_id, controller.id)
            .exec()
            .await?;

        // Delete the original command message.
        cc.http
            .delete_message(cc.msg.channel_id, cc.msg.id)
            .exec()
            .await?;

        // Nothing more to be done here.
        return Ok(None);
    }

    let resp = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content(
                    "Done; You can use `bot edit` command to edit the message content, or `roles \
                     edit` command to edit the role mappings."
                        .to_string(),
                )
                .build(),
        ),
    };

    interaction
        .create_response(controller_mci.id, &controller_mci.token, &resp)
        .exec()
        .await?;

    // Delete the controller message.
    cc.http
        .delete_message(controller.channel_id, controller.id)
        .exec()
        .await?;

    // Delete the original command message.
    cc.http
        .delete_message(cc.msg.channel_id, cc.msg.id)
        .exec()
        .await
        .ok(); // Ignore outcome.

    Ok(Some(mappings))
}

async fn add_reactions_to_message(
    cc: &CommandContext<'_>,
    mappings: &[ReactionRole],
    message: &Message,
) -> AnyResult<()> {
    for rr in mappings.iter() {
        let request_emoji = request_from_emoji(&rr.emoji);

        cc.http
            .create_reaction(message.channel_id, message.id, &request_emoji)
            .exec()
            .await?;
    }

    Ok(())
}

/// Creates components for reaction-roles setup dropdown selection.
async fn dropdown_components(
    cc: &CommandContext<'_>,
    guild_id: Id<GuildMarker>,
    reaction: &Reaction,
) -> Result<Vec<Component>, CommandError> {
    // Get all available roles, try cache.
    let roles = cc.cache.guild_roles(guild_id).and_then(|role_ids| {
        let mut cached_roles = Vec::with_capacity(role_ids.len());

        for id in role_ids.iter() {
            // Return out of closure if role is not cached.
            cc.cache
                .role(*id)
                .map(|r| cached_roles.push(r.resource().to_owned()))?
        }

        // All roles were cached.
        Some(cached_roles)
    });

    // Use cached roles or otherwise fetch from client.
    let roles = match roles {
        Some(r) => r,
        None => {
            let fetch = cc.http.roles(guild_id).send().await?;

            // Manually update the cache.
            for role in fetch.iter().cloned() {
                cc.cache.update(&RoleUpdate { guild_id, role });
            }

            fetch
        },
    };

    // Find the highest role that the bot has.
    let bot_role = {
        let bot_roles = match cc.cache.member(guild_id, cc.user.id) {
            Some(m) => m.value().roles().to_vec(),
            None => {
                cc.http
                    .guild_member(guild_id, cc.user.id)
                    .send()
                    .await?
                    .roles
            },
        };

        roles
            .iter()
            .filter(|r| bot_roles.contains(&r.id))
            .max()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Could not find maximum role for bot"))?
    };

    // Role options to display.
    let role_opts = roles
        .into_iter()
        // Filter out `@everyone` role.
        .filter(|r| r.id.cast() != guild_id)
        // Filter out roles that are higher or same as the bot's role.
        .filter(|r| *r < bot_role)
        // Filter out roles that are integration managed.
        .filter(|r| !r.managed)
        // Filter out roles with admin permissions, as a precaution.
        .filter(|r| !r.permissions.contains(Permissions::ADMINISTRATOR))
        .map(|r| SelectMenuOption {
            default: false,
            description: None,
            emoji: Some(reaction.emoji.to_owned()),
            label: r.name,
            value: r.id.to_string(),
        }).chain([
            SelectMenuOption {
                default: false,
                description: Some("Cancel".to_string()),
                emoji: None,
                label: " ".to_string(), // Empty, but not.
                value: "cancel".to_string(),
            }
        ])
        .collect::<Vec<_>>();

    // Roles dropdown list.
    Ok(vec![Component::ActionRow(ActionRow {
        components: vec![Component::SelectMenu(SelectMenu {
            custom_id: "role".to_string(),
            disabled: false,
            max_values: Some(1),
            min_values: Some(1),
            options: role_opts,
            placeholder: Some("Select a role to add".to_string()),
        })],
    })])
}

fn controller_components(enabled: bool) -> Vec<Component> {
    vec![Component::ActionRow(ActionRow {
        components: vec![
            // Button to finish adding reactions.
            Component::Button(Button {
                custom_id: Some("roles_done".to_string()),
                disabled: !enabled,
                emoji: None,
                label: Some("Done".to_string()),
                style: ButtonStyle::Success,
                url: None,
            }),
            // Button to cancel the process.
            Component::Button(Button {
                custom_id: Some("roles_cancel".to_string()),
                disabled: !enabled,
                emoji: None,
                label: Some("Cancel".to_string()),
                style: ButtonStyle::Danger,
                url: None,
            }),
        ],
    })]
}

/// Enable or disable setup message controls.
/// If `content` is `None`, previous content is used.
async fn update_controller(
    cc: &CommandContext<'_>,
    controller: &mut Message,
    content: Option<&str>,
    enabled: bool,
) -> AnyResult<()> {
    *controller = cc
        .http
        .update_message(controller.channel_id, controller.id)
        .content(content.or(Some(&controller.content)))?
        .components(Some(&controller_components(enabled)))?
        .send()
        .await?;

    Ok(())
}

async fn display_emoji_roles(
    cc: &CommandContext<'_>,
    guild_id: Id<GuildMarker>,
    emoji_roles: &[ReactionRole],
) -> Result<String, CommandError> {
    let mut emoji_roles_msg = String::new();

    for ReactionRole { emoji, role } in emoji_roles {
        let (Ok(emoji) | Err(emoji)) = utils::display_reaction_emoji(emoji);

        emoji_roles_msg.push_str(&emoji);
        emoji_roles_msg.push_str(" : `");

        // Try to get a name from the cache.
        let cached_name = cc.cache.role(*role).map(|r| r.name.to_string());
        let name = match cached_name {
            Some(n) => n,
            None => {
                let roles = cc.http.roles(guild_id).send().await?;

                // Name of this role.
                let this = roles
                    .iter()
                    .find(|r| r.id == *role)
                    .map(|r| r.name.to_string())
                    .unwrap_or_else(|| role.to_string()); // Use id, if all else fails.

                // Manually update the cache.
                for role in roles {
                    cc.cache.update(&RoleUpdate { guild_id, role });
                }

                this
            },
        };

        emoji_roles_msg.push_str(&name);
        emoji_roles_msg.push_str("`\n");
    }

    Ok(emoji_roles_msg)
}

fn request_from_emoji(r: &ReactionType) -> RequestReactionType {
    match r {
        ReactionType::Custom { id, name, .. } => RequestReactionType::Custom {
            id: *id,
            name: name.as_deref(),
        },
        ReactionType::Unicode { name } => RequestReactionType::Unicode { name },
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReactionRole {
    pub emoji: ReactionType,
    pub role: Id<RoleMarker>,
}

impl ReactionRole {
    pub const fn new(emoji: ReactionType, role: Id<RoleMarker>) -> Self {
        Self { emoji, role }
    }
}

impl PartialEq for ReactionRole {
    fn eq(&self, other: &Self) -> bool {
        utils::reaction_type_eq(&self.emoji, &other.emoji) && self.role == other.role
    }
}
