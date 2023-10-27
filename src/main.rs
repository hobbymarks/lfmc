use anyhow::{anyhow, Result};
use clap::Parser;
use dotenv;
use log::{error, info};
use reqwest;
use serde_json::Value;
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Your Last.fm API Key
    #[arg(short = 'k', long, env = "API_KEY")]
    api_key: String,

    /// Your Last.fm Username
    #[arg(short, long, env = "USERNAME")]
    username: String,

    /// The limit of Artists
    #[arg(short, long, default_value = "5", env = "LIMIT")]
    limit: u16,

    /// The lookback period
    #[arg(short, long, default_value = "7day", env = "PERIOD")]
    period: String,
}

struct Config {
    api_key: String,
    username: String,
    limit: u16,
    period: String,
}

impl Config {
    fn new(api_key: String, username: String, limit: u16, period: String) -> Self {
        Config {
            api_key,
            username,
            limit,
            period,
        }
    }

    fn get_uri(&self) -> String {
        format!(
            "http://ws.audioscrobbler.com/{}/?method={}&user={}&api_key={}&format={}&period={}&limit={}",
            "2.0",
            "user.gettopartists",
            &self.username,
            &self.api_key,
            "json",
            &self.period,
            &self.limit,
        )
    }
}

fn construct_output(config: Config, json: Value) -> Result<String> {
    let period: &str = match config.period.as_str() {
        "overall" => "",
        "7day" => " week",
        "1month" => " month",
        "3month" => " 3 months",
        "6month" => " 6 months",
        "12month" => " year",
        _ => return Err(anyhow!("Period {} not allowed. Only allow \"overall\", \"7day\", \"1month\", \"3month\", \"6month\", or \"12month\".", config.period))
    };
    info!("period={}", period);

    let mut output: String = format!(
        "♫ My Top {} played artists in the past{} via #LastFM ♫:\n",
        config.limit.to_string(),
        period
    );
    info!("output={}", output);

    let artists = json["topartists"]["artist"]
        .as_array()
        .ok_or(anyhow!("Error parsing JSON."))?;

    for (i, artist) in artists.iter().enumerate() {
        info!("i={},artist={}", i, artist);
        let ending = match i {
            x if x <= (config.limit as usize - 3) => ",",
            x if x == (config.limit as usize - 2) => ", &",
            _ => "",
        };

        let name = artist["name"]
            .as_str()
            .ok_or(anyhow!("Artist not found."))?;
        let playcount = artist["playcount"]
            .as_str()
            .ok_or(anyhow!("Playcount not found."))?;

        output = format!(" {} {} ({}){}", output, name, playcount, ending);
        info!("output={}", output);
    }

    info!("output={}", output);
    Ok(format!("{}.", output))
}

#[cfg(test)]
mod tests {
    use log::info;
    use serde_json::Value;

    use crate::{construct_output, Config};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_config() {
        init();

        let api_key = "api_key";
        let username = "username";
        let limit = 5;
        let period = "7day";

        let config = Config::new(
            String::from(api_key),
            String::from(username),
            limit,
            String::from(period),
        );

        let uri = config.get_uri();
        info!("uri={}", uri);

        let keys = [
            format!("user={}", username),
            format!("api_key={}", api_key),
            format!("limit={}", limit),
            format!("period={}", period),
        ];
        for pat in keys.iter() {
            assert!(uri.find(pat).is_some());
        }
    }

    #[test]
    fn test_construct_output() {
        init();

        let api_key = "api_key";
        let username = "username";
        let limit = 5;
        let period = "7day";

        let config = Config::new(
            String::from(api_key),
            String::from(username),
            limit,
            String::from(period),
        );

        let artist = r#"
        {
            "topartists":{
                "artist":["Fia","Sea","Tha","Foa","Fia"]}
        }
        "#;

        let parsed_json: Result<Value, serde_json::Error> = serde_json::from_str(artist);

        if let Ok(json) = parsed_json {
            let output: Result<String, anyhow::Error> = construct_output(config, json);
            if let Ok(output_string) = output {
                let key = "Fia";
                assert!(output_string.find(key).is_some());
            }
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}]{}:{} {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .init();

    info!(" main running ... ");

    if let Some(home_dir) = dirs::home_dir() {
        info!("Loading env ...");
        dotenv::from_filename(format!("{}/.config/lfmc/.env", home_dir.to_string_lossy())).ok();
    }

    info!("Parsing args ...");
    let args = Args::parse();

    info!("Creating config ...");
    let config = Config::new(args.api_key, args.username, args.limit, args.period);

    let resp: Result<_, reqwest::Error> = reqwest::blocking::get(config.get_uri())?.json::<Value>();

    if let Ok(json) = resp {
        info!("Constructing output ...");
        let output = construct_output(config, json)?;
        println!("\n{}\n", output);
    } else {
        error!("Could not convert response to JSON.");
        return Err(anyhow!("Could not convert response to JSON."));
    }

    info!("main finished.");
    Ok(())
}
