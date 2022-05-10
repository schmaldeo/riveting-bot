use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::select_menu::SelectMenuOption;
use twilight_model::application::component::text_input::TextInputStyle;
use twilight_model::application::component::{ActionRow, Button, Component, SelectMenu, TextInput};
use twilight_model::application::interaction::MessageComponentInteraction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::ReactionType;
use twilight_model::gateway::payload::incoming::{MessageCreate, ReactionAdd};
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::RoleMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::{CommandContext, CommandError, CommandResult};
use crate::utils::prelude::*;

/// Command: Setup a reaction-roles message.
pub async fn roles(cc: CommandContext<'_>) -> CommandResult {
    let Some(guild_id) = cc.msg.guild_id else {
        return Err(CommandError::Disabled)
    };

    let done_btn = Component::Button(Button {
        custom_id: Some("roles_done".to_string()),
        disabled: false,
        emoji: None,
        label: Some("Done".to_string()),
        style: ButtonStyle::Success,
        url: None,
    });

    let cancel_btn = Component::Button(Button {
        custom_id: Some("roles_cancel".to_string()),
        disabled: false,
        emoji: None,
        label: Some("Cancel".to_string()),
        style: ButtonStyle::Danger,
        url: None,
    });

    let components = vec![Component::ActionRow(ActionRow {
        components: vec![done_btn, cancel_btn],
    })];

    let mut controller = cc
        .http
        .create_message(cc.msg.channel_id)
        .reply(cc.msg.id)
        .components(&components)?
        .send()
        .await?;

    let author_id = cc.msg.author.id;

    let mut emoji_roles = Vec::new();

    let controller_mci = loop {
        let controller_fut = cc
            .standby
            .wait_for_component(controller.id, move |event: &MessageComponentInteraction| {
                event.author_id() == Some(author_id)
            });

        let reaction_fut = cc
            .standby
            .wait_for_reaction(controller.id, move |event: &ReactionAdd| {
                event.user_id == author_id
            });

        let reaction = tokio::select! {
            biased;
            r = reaction_fut => r?,
            c = controller_fut => break c?,
        };

        let roles = cc.http.roles(guild_id).send().await?;

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

        let role_list = Component::SelectMenu(SelectMenu {
            custom_id: "role".to_string(),
            disabled: false,
            max_values: Some(1),
            min_values: Some(1),
            options: role_opts,
            placeholder: Some("Select a role".to_string()),
        });

        // let add_btn = Component::Button(Button {
        //     custom_id: Some("roles_add".to_string()),
        //     disabled: true,
        //     emoji: None,
        //     label: Some("Add".to_string()),
        //     style: ButtonStyle::Primary,
        //     url: None,
        // });

        let components = vec![
            Component::ActionRow(ActionRow {
                components: vec![role_list],
            }),
            // Component::ActionRow(ActionRow {
            //     components: vec![add_btn],
            // }),
        ];

        let res = cc
            .http
            .create_message(cc.msg.channel_id)
            .reply(cc.msg.id)
            .components(&components)?
            .exec()
            .await?;

        let model = res.model().await?;

        let mut list_mci = cc
            .standby
            .wait_for_component(model.id, move |event: &MessageComponentInteraction| {
                event.author_id() == Some(author_id)
            })
            .await?;

        emoji_roles.push((
            reaction.emoji.to_owned(),
            list_mci
                .data
                .values
                .pop()
                .unwrap()
                .parse::<Id<RoleMarker>>()
                .unwrap(),
        ));

        let mut emoji_roles_msg = String::new();
        emoji_roles_msg.push_str("```");
        for (emoji, role) in emoji_roles.iter() {
            let emoji = match emoji {
                ReactionType::Custom { id, name, .. } => match name {
                    Some(n) => n.to_string(),
                    None => id.to_string(),
                },
                ReactionType::Unicode { name } => name.to_string(),
            };
            emoji_roles_msg.push_str(&emoji);
            emoji_roles_msg.push_str(" : ");
            emoji_roles_msg.push_str(&role.to_string());
            emoji_roles_msg.push('\n');
        }
        emoji_roles_msg.push_str("```");

        controller = cc
            .http
            .update_message(controller.channel_id, controller.id)
            .content(Some(&emoji_roles_msg))?
            .send()
            .await?;

        let inter = cc.http.interaction(cc.application.id);

        let resp = InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: Some(InteractionResponseData::default()),
        };

        inter
            .create_response(list_mci.id, &list_mci.token, &resp)
            .exec()
            .await?;

        inter.delete_response(&list_mci.token).exec().await?;
    };

    let inter = cc.http.interaction(cc.application.id);

    let resp = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .content("Done; Reply to this message to set its content.".to_string())
                .build(),
        ),
    };

    inter
        .create_response(controller_mci.id, &controller_mci.token, &resp)
        .exec()
        .await?;

    inter
        .update_response(&controller_mci.token)
        .content(Some("content"))?
        .send()
        .await?;

    let followup = inter
        .create_followup(&controller_mci.token)
        .flags(MessageFlags::EPHEMERAL)
        .content("content")?
        .send()
        .await?;

    println!("{:?}", followup);

    Ok(())
}
