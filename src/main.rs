use core::error;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use reqwest::Response;
use serde::Deserialize;
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone)]
struct Coin {
    id: String,
    name: String,
    symbol: String,
    current_price: f64,
    price_change_24h: f64,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoData {
    usd: f64,
    usd_24h_change: f64,
}

impl Coin {
    fn price_formatted(&self) -> String {
        format!("${:.2}", self.current_price)
    }

    fn change_24h_formatted(&self) -> String {
        format!("{:+.2}%", self.price_change_24h * 100.0)
    }

    fn is_up(&self) -> bool {
        self.price_change_24h > 0.0
    }
}

async fn fetch_coin_prices() -> Result<String, reqwest::Error> {
    // 1. Define the API URL
    let api_url: String = "https://api.coingecko.com/api/v3/simple/price?".to_string();
    // Hard coded values for now
    let coin_ids: String = "ids=bitcoin,ethereum,cardano&".to_string();
    let vs_currency: String = "vs_currencies=usd&".to_string();
    let include_24_hour_change: String = "include_24hr_change=true".to_string();

    let url: String = format!(
        "{}{}{}{}",
        api_url, coin_ids, vs_currency, include_24_hour_change
    );
    // 2. Make HTTP GET request
    let response: Response = reqwest::get(url).await?;

    // 3. Get response text
    let response_text = response.text().await?;

    Ok(response_text)
}

fn parse_coin_response(
    json_text: &str,
) -> Result<HashMap<String, CoinGeckoData>, serde_json::Error> {
    let parsed: HashMap<String, CoinGeckoData> = serde_json::from_str(json_text)?;
    Ok(parsed)
}

fn convert_to_coins(coin_map: HashMap<String, CoinGeckoData>) -> Vec<Coin> {
    let mut coins: Vec<Coin> = Vec::new();

    for (coin_id, coin_data) in coin_map {
        let coin = Coin {
            id: coin_id.clone(),
            symbol: coin_id.to_uppercase(),
            name: coin_id.clone(),
            current_price: coin_data.usd,
            price_change_24h: coin_data.usd_24h_change,
        };
        coins.push(coin);
    }
    coins
}

/*
fn get_sample_coins() -> Vec<Coin> {
    vec![
        Coin {
            id: "bitcoin".to_string(),
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            current_price: 11000.320,
            price_change_24h: -0.05,
        },
        Coin {
            id: "ethereum".to_string(),
            name: "Ethereum".to_string(),
            symbol: "ETH".to_string(),
            current_price: 6000.23,
            price_change_24h: -0.05,
        },
        Coin {
            id: "cardano".to_string(),
            name: "Cardano".to_string(),
            symbol: "ADA".to_string(),
            current_price: 672.320,
            price_change_24h: 0.27,
        },
    ]
}
*/

fn format_coins(coins: &[Coin]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for coin in coins {
        let line = format!(
            "{:6} {:12} ${:>10.2} {:>6.2}%",
            coin.symbol, coin.name, coin.current_price, coin.price_change_24h
        );
        lines.push(line);
    }
    lines.join("\n")
}

async fn refresh_output() -> Result<Vec<Coin>, Box<dyn std::error::Error>> {
    let json_text = fetch_coin_prices().await?;
    let coin_map = parse_coin_response(&json_text)?;
    Ok(convert_to_coins(coin_map))
}

fn ui(frame: &mut Frame, coins: &[Coin]) {
    // Create the area
    // Then split the area into chunks
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header (fixed 1 line)
            Constraint::Min(1),    // Main area (grows)
            Constraint::Length(1), // Footer (fixed 1 line)
        ])
        .split(area);

    let header_area = chunks[0];
    let main_area = chunks[1];
    let footer_area = chunks[2];

    // HEADER
    let header = Block::default().title("Crypto Tracker");
    frame.render_widget(header, header_area);

    // MAIH
    // Initial Refresh
    let text = format_coins(&coins);
    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, main_area);

    // FOOTER
    let help_message = Paragraph::new("Press 'q' to quit");

    frame.render_widget(help_message, footer_area);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up terminal
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let mut coins = refresh_output().await?;

    // Event loop
    loop {
        terminal.draw(|frame| {
            ui(frame, &coins); // call the custom UI function
        })?;

        // Poll for events with timeout
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Clean up
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
