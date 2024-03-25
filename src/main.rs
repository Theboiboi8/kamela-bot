use anyhow::Context as _;
use serenity::all::{Command, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction};
use serenity::async_trait;
use serenity::gateway::ActivityData;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use tracing::{info};

mod weather;

struct Bot {
    weather_api_key: String,
    client: reqwest::Client,
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

	    let _commands_global =
		    Command::set_global_commands(&context.http, commands_vec.clone());

        info!("Registered commands: {:#?}", commands_vec.clone());
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let response_content = match command.data.name.as_str() {
                "info" => {
	                CreateEmbed::new()
		                .colour(0xDFFF00)
		                .description("A general-purpose Discord bot.")
		                .footer(CreateEmbedFooter::new(
			                format!("Kamela Bot v{}", env!("CARGO_PKG_VERSION"))
		                ))
		                .title("Info")
                },
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
				            CreateEmbed::new()
					            .colour(0xDFFF00)
					            .description(
						            format!("Forecast: {} in {}", forecast.headline.overview, location)
					            )
					            .footer(CreateEmbedFooter::new(
						            format!("Kamela Bot v{}", env!("CARGO_PKG_VERSION"))
					            ))
					            .title(format!("Weather: {place}"))
			            }
			            Err(err) => {
				            CreateEmbed::new()
					            .colour(0xFF0037)
					            .description(format!("Error: {err}"))
					            .footer(CreateEmbedFooter::new(
						            format!("Kamela Bot v{}", env!("CARGO_PKG_VERSION"))
					            ))
					            .title("Error!")
			            }
		            }
	            }
                command => unreachable!("Unknown command: {}", command),
            };

            let data =
	            CreateInteractionResponseMessage::new().embed(response_content);
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
	
    let client = get_client(
        &discord_token,
        &weather_api_key,
    ).await;
    Ok(client.into())
}

#[allow(clippy::missing_panics_doc)]
pub async fn get_client(
	discord_token: &str,
	weather_api_key: &str,
) -> Client {
    // Set gateway intents, which decides what events the bot will be notified about.
    // Here we don't need any intents so empty
    let intents = GatewayIntents::default();

    Client::builder(discord_token, intents)
        .event_handler(Bot {
            weather_api_key: weather_api_key.to_owned(),
            client: reqwest::Client::new(),
        })
	    .activity(
		    ActivityData::custom("General Purpose Discord Bot")
	    )
        .await
        .expect("Error creating client")
}