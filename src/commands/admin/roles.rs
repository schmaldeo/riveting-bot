use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::select_menu::SelectMenuOption;
use twilight_model::application::component::{ActionRow, Button, Component, SelectMenu};
use twilight_model::application::interaction::MessageComponentInteraction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::ReactionType;
use twilight_model::gateway::payload::incoming::{ReactionAdd, RoleUpdate};
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::RoleMarker;
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::{CommandContext, CommandError, CommandResult};
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

    let components = vec![Component::ActionRow(ActionRow {
        components: vec![
            // Button to finish adding reactions.
            Component::Button(Button {
                custom_id: Some("roles_done".to_string()),
                disabled: false,
                emoji: None,
                label: Some("Done".to_string()),
                style: ButtonStyle::Success,
                url: None,
            }),
            // Button to cancel the process.
            Component::Button(Button {
                custom_id: Some("roles_cancel".to_string()),
                disabled: false,
                emoji: None,
                label: Some("Cancel".to_string()),
                style: ButtonStyle::Danger,
                url: None,
            }),
        ],
    })];

    // Setup message with controls.
    let mut controller = cc
        .http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .content("React to this message with an emoji to add reaction-roles.")?
        .components(&components)?
        .send()
        .await?;

    let interaction = cc.http.interaction(cc.application.id);
    let author_id = cc.msg.author.id;
    let mut emoji_roles = Vec::new();

    let controller_mci = loop {
        // Future that waits for controller button press.
        let controller_fut = cc
            .standby
            .wait_for_component(controller.id, move |event: &MessageComponentInteraction| {
                event.author_id() == Some(author_id)
            });

        // Future that waits for a reaction.
        let reaction_fut = cc
            .standby
            .wait_for_reaction(controller.id, move |event: &ReactionAdd| {
                event.user_id == author_id
            });

        // Wait for a reaction or a controller button.
        let reaction = tokio::select! {
            biased;
            r = reaction_fut => r?, // Proceed with the reaction.
            c = controller_fut => break c?, // Exit loop with button interaction.
        };

        // Get all available roles.
        let roles = cc.cache.guild_roles(guild_id).and_then(|role_ids| {
            let mut roles = Vec::with_capacity(role_ids.len());

            for id in role_ids.iter() {
                // Return out of closure if role is not cached.
                cc.cache
                    .role(*id)
                    .map(|r| roles.push(r.resource().to_owned()))?
            }

            // All roles were cached.
            Some(roles)
        });

        // Use cached roles or otherwise fetch from client.
        let roles = match roles {
            Some(r) => r,
            None => cc.http.roles(guild_id).send().await?,
        };

        let role_opts = roles
            .into_iter()
            // Filter out `@everyone` role.
            .filter(|r| r.id.cast() != guild_id)
            // Filter out roles with admin permissions, as a precaution.
            .filter(|r| !r.permissions.contains(Permissions::ADMINISTRATOR))
            .map(|r| SelectMenuOption {
                default: false,
                description: None,
                emoji: Some(reaction.emoji.to_owned()),
                label: r.name,
                value: r.id.to_string(),
            })
            .collect();

        // Roles drop-down list.
        let components = vec![Component::ActionRow(ActionRow {
            components: vec![Component::SelectMenu(SelectMenu {
                custom_id: "role".to_string(),
                disabled: false,
                max_values: Some(1),
                min_values: Some(1),
                options: role_opts,
                placeholder: Some("Select a role".to_string()),
            })],
        })];

        let drop_down = cc
            .http
            .create_message(cc.msg.channel_id)
            .reply(cc.msg.id)
            .components(&components)?
            .send()
            .await?;

        // Wait for user to select an option.
        let mut list_mci = cc
            .standby
            .wait_for_component(drop_down.id, move |event: &MessageComponentInteraction| {
                event.author_id() == Some(author_id)
            })
            .await?;

        // Save the choice.
        emoji_roles.push((
            reaction.emoji.to_owned(),
            list_mci
            .data
            .values
            .pop()
            .unwrap() // FIXME
            .parse::<Id<RoleMarker>>()
            .unwrap(), // FIXME
        ));

        let resp = InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: Some(InteractionResponseData::default()),
        };

        // Acknowledge the interaction.
        interaction
            .create_response(list_mci.id, &list_mci.token, &resp)
            .exec()
            .await?;

        // Delete the drop-down message.
        interaction.delete_response(&list_mci.token).exec().await?;

        // Create a message that lists all roles that have been added so far.
        let mut emoji_roles_msg = String::new();

        for (emoji, role) in emoji_roles.iter() {
            let emoji = match emoji {
                ReactionType::Custom { id, name, .. } => match name {
                    Some(n) => format!(":{n}:"),
                    None => id.to_string(), // This should only happen if emoji was deleted from the guild, or something.
                },
                ReactionType::Unicode { name } => name.to_string(),
            };

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

        // Update the controller message.
        controller = cc
            .http
            .update_message(controller.channel_id, controller.id)
            .content(Some(&emoji_roles_msg))?
            .send()
            .await?;
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
        return Ok(());
    }

    // If no reaction-roles were added.
    if emoji_roles.is_empty() {
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
        return Ok(());
    }

    let resp = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content("Done; Reply to the output message to set its content.".to_string())
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
        .await?;

    let output = cc
        .http
        .create_message(cc.msg.channel_id)
        .content(&controller.content)?
        .send()
        .await?;

    let mut map = Vec::new();

    for (emoji, role) in emoji_roles {
        let request_emoji = match emoji {
            ReactionType::Custom { id, ref name, .. } => RequestReactionType::Custom {
                id,
                name: name.as_deref(),
            },
            ReactionType::Unicode { ref name } => RequestReactionType::Unicode { name },
        };

        cc.http
            .create_reaction(output.channel_id, output.id, &request_emoji)
            .exec()
            .await?;

        map.push(ReactionRole::new(emoji, role));
    }

    let mut lock = cc.config.lock().unwrap();
    lock.add_reaction_roles(guild_id, output.channel_id, output.id, map);

    lock.write_guild(guild_id)?;

    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReactionRole {
    pub emoji: ReactionType,
    pub role: Id<RoleMarker>,
}

impl ReactionRole {
    pub fn new(emoji: ReactionType, role: Id<RoleMarker>) -> Self {
        Self { emoji, role }
    }
}
