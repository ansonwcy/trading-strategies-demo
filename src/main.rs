use std::fs::File;
use std::io::{BufReader, BufRead};
use serde::{Deserialize, Serialize};
use trading_strategies::{Strategy, TickData};
use trading_strategies::core::{TradeEvent, TradeObserver};
use trading_strategies::core::tick_strategy::TickStrategyWrapper;
use trading_strategies::prelude::{StochasticStrategy, StochasticConfig};

#[derive(Debug, Serialize, Deserialize)]
struct TickRecord {
    timestamp: i64,
    price: f64,
    volume: f64,
}

// Wrapper to convert seconds to milliseconds for the library
struct TickWrapper<'a> {
    tick: &'a TickRecord,
}

impl<'a> TickData for TickWrapper<'a> {
    fn price(&self) -> f64 {
        self.tick.price
    }
    
    fn volume(&self) -> f64 {
        self.tick.volume
    }
    
    fn timestamp(&self) -> i64 {
        // Convert seconds to milliseconds
        self.tick.timestamp * 1000
    }
    
    fn symbol(&self) -> &str {
        "BTC/USD"
    }
}

// Trade logger observer - logs all trades to console
struct TradeLogger {
    trade_count: usize,
}

impl TradeLogger {
    fn new() -> Self {
        Self { trade_count: 0 }
    }
}

impl TradeObserver for TradeLogger {
    fn on_trade(&mut self, event: TradeEvent) {
        self.trade_count += 1;
        match event {
            TradeEvent::Buy(trade) => {
                println!("\n🟢 BUY SIGNAL #{} - Trade Observer Notification:", self.trade_count);
                println!("   Price: ${:.2}", trade.entry_price);
                println!("   Quantity: {:.4}", trade.quantity);
                println!("   Time: {}", trade.entry_time);
            }
            TradeEvent::Sell(trade) => {
                println!("\n🔴 SELL SIGNAL #{} - Trade Observer Notification:", self.trade_count);
                println!("   Entry: ${:.2} -> Exit: ${:.2}", trade.entry_price, trade.exit_price);
                println!("   P&L: ${:.2} ({:.2}%)", trade.pnl, trade.pnl_percentage);
                println!("   Time: {}", trade.exit_time);
            }
        }
    }
}

fn main() {
    // Read tick data from file
    let file = File::open("tick_strategy_ticks.jsonl").expect("Failed to open tick file");
    let reader = BufReader::new(file);
    
    let mut ticks: Vec<TickRecord> = Vec::new();
    for line in reader.lines() {
        if let Ok(line_str) = line {
            if let Ok(tick) = serde_json::from_str::<TickRecord>(&line_str) {
                ticks.push(tick);
            }
        }
    }
    
    // Configure stochastic strategy
    let config = StochasticConfig {
        k_period: 5,      // Short period for limited data
        d_period: 3,
        oversold_threshold: 20.0,
        overbought_threshold: 80.0,
        position_size: 1.0,
        atr_period: 5,
        atr_multiplier: 2.0,
    };
    
    // Create strategy with initial cash
    let initial_cash = 10000.0;
    let mut strategy = StochasticStrategy::new(config, initial_cash);
    
    // Register observer for trade logging
    let trade_logger = TradeLogger::new();
    strategy.add_observer(Box::new(trade_logger));
    
    // Use the built-in TickStrategyWrapper 
    let timeframe_secs = 1;
    let mut tick_wrapper = TickStrategyWrapper::new(strategy, timeframe_secs);
    
    // Process all ticks
    for tick in ticks.iter() {
        let wrapped_tick = TickWrapper { tick };
        tick_wrapper.process_tick(&wrapped_tick);
    }
    
    // Force close any pending candle
    if let Some(last_tick) = ticks.last() {
        tick_wrapper.force_close_candle(last_tick.timestamp * 1000);
    }
    
    // Final summary
    let final_price = ticks.last().map(|t| t.price).unwrap_or(3000.0);
    let final_equity = tick_wrapper.strategy().calculate_equity(final_price);
    let pnl = final_equity - initial_cash;
    let pnl_pct = (pnl / initial_cash) * 100.0;
    
    println!("\n=== Final Summary ===");
    println!("P&L: ${:.2} ({:.2}%)", pnl, pnl_pct);
}