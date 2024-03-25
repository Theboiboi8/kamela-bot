mod weather;

use anyhow::Context as _;
use serenity::all::{ApplicationId, Command, CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId, Interaction};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use tracing::{error, info};

struct Bot {
    weather_api_key: String,
    client: reqwest::Client,
    discord_guild_id: GuildId,
	discord_application_id: ApplicationId,
}

#[async_trait]
#[allow(clippy::unreadable_literal)]
impl EventHandler for Bot {
    async fn ready(&self, context : Context, ready : Ready) {
        info!("{} is connected!", ready.user.name);

        let commands_vec = vec![
            CreateCommand::new("info").description("Info about Kamela Bot"),
	        CreateCommand::new("weather")
		        .description("Returns information about the weather")
		        .add_option(
			        CreateCommandOption::new(
				        serenity::all::CommandOptionType::String,
				        "place",
				        "City to return weather for"
			        )
		        )
        ];

        let _commands_guild = &self
	        .discord_guild_id
	        .set_commands(&context.http, commands_vec.clone())
	        .await
	        .unwrap();

	    let _commands_global =
		    Command::set_global_commands(&context.http, commands_vec.clone());

        info!("Registered commands: {:#?}", commands_vec);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let response_content = match command.data.name.as_str() {
                "info" => format!("Kamela Bot v{}", env!("CARGO_PKG_VERSION")).to_owned(),
	            "weather" => {
		            let argument = command
			            .data
			            .options
			            .iter()
			            .find(|opt| opt.name == "place")
			            .cloned();
		            let value = argument.unwrap().value;
		            let place = value.as_str().unwrap();
		            let result =
			            weather::get_forecast(place, &self.weather_api_key, &self.client).await;
		            match result {
			            Ok((location, forecast)) => {
				            format!("Forecast: {} in {}", forecast.headline.overview, location)
			            }
			            Err(err) => {
				            format!("Error: {err}")
			            }
		            }
	            }
                command => unreachable!("Unknown command: {}", command),
            };

            let data = CreateInteractionResponseMessage::new().content(response_content);
            let builder = CreateInteractionResponse::Message(data);

            if let Err(why) = command.create_response(&ctx.http, builder).await {
                println!("Cannot respond to slash command: {why}");
            }
        }
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let discord_token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let weather_api_key = secrets
        .get("WEATHER_API_KEY")
        .context("'WEATHER_API_KEY' was not found")?;

    let discord_guild_id = secrets
        .get("DISCORD_GUILD_ID")
        .context("'DISCORD_GUILD_ID' was not found")?;

	let discord_application_id = secrets
		.get("DISCORD_APPLICATION_ID")
		.context("'DISCORD_APPLICATION_ID' was not found")?;

    let client = get_client(
        &discord_token,
        &weather_api_key,
        discord_guild_id.parse().unwrap(),
	    discord_application_id.parse().unwrap(),
    ).await;
    Ok(client.into())
}

#[allow(clippy::missing_panics_doc)]
pub async fn get_client(
    discord_token: &str,
    weather_api_key: &str,
    discord_guild_id: u64,
	discord_application_id: u64,
) -> Client {
    // Set gateway intents, which decides what events the bot will be notified about.
    // Here we don't need any intents so empty
    let intents = GatewayIntents::empty();

    Client::builder(discord_token, intents)
        .event_handler(Bot {
            weather_api_key: weather_api_key.to_owned(),
            client: reqwest::Client::new(),
            discord_guild_id: GuildId::new(discord_guild_id),
	        discord_application_id: ApplicationId::new(discord_application_id)
        })
        .await
        .expect("Err creating client")
}