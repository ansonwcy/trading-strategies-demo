use std::fs::File;
use std::io::{BufReader, BufRead};
use serde::{Deserialize, Serialize};
use trading_strategies::{Strategy, TickData};
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


fn main() {
    println!("Loading tick data...");
    
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
    
    println!("Loaded {} ticks", ticks.len());
    
    // Analyze tick data to determine appropriate timeframe
    if ticks.len() >= 2 {
        let duration = ticks.last().unwrap().timestamp - ticks.first().unwrap().timestamp;
        println!("Data spans {} seconds ({:.1} hours)", duration, duration as f64 / 3600.0);
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
    let strategy = StochasticStrategy::new(config, initial_cash);
    
    // Use the built-in TickStrategyWrapper 
    // Set timeframe to 1 second to effectively treat each tick as a candle
    let timeframe_secs = 1; // 1 second candles = essentially tick-as-candle for this sparse data
    let mut tick_wrapper = TickStrategyWrapper::new(strategy, timeframe_secs);
    
    println!("\nProcessing ticks through TickStrategyWrapper ({}s candles)...", timeframe_secs);
    
    // Track candles and trades
    let mut candle_count = 0;
    let mut _last_candle_timestamp = None;
    
    for (i, tick) in ticks.iter().enumerate() {
        // Get current candle before processing
        let before_candle = tick_wrapper.get_current_candle();
        
        // Process the tick - this will automatically build candles
        let wrapped_tick = TickWrapper { tick };
        tick_wrapper.process_tick(&wrapped_tick);
        
        // Check if a new candle was completed
        let after_candle = tick_wrapper.get_current_candle();
        
        // Detect candle completion
        if let Some(before) = before_candle {
            if after_candle.is_none() || 
               (after_candle.is_some() && after_candle.as_ref().unwrap().timestamp != before.timestamp) {
                candle_count += 1;
                
                // Show candle details for first few and then periodically
                if candle_count <= 5 || candle_count % 10 == 0 {
                    println!("\nCandle {}: OHLC [{:.2}, {:.2}, {:.2}, {:.2}]", 
                        candle_count, before.open, before.high, before.low, before.close);
                }
                
                _last_candle_timestamp = Some(before.timestamp);
            }
        }
        
        // Show progress and diagnostics
        if (i + 1) % 50 == 0 {
            println!("\nProcessed {} ticks, {} candles formed", i + 1, candle_count);
            
            let trades = tick_wrapper.strategy().get_trades();
            if !trades.is_empty() {
                println!("Total trades: {}", trades.len());
            }
            
            // Show indicator values
            if let (Some(k), Some(d)) = (tick_wrapper.strategy().current_k, tick_wrapper.strategy().current_d) {
                println!("Stochastic K: {:.2}, D: {:.2}", k, d);
                if k < 20.0 {
                    println!(">> In OVERSOLD zone");
                } else if k > 80.0 {
                    println!(">> In OVERBOUGHT zone");
                }
            }
            
            // Show open positions
            let positions = tick_wrapper.strategy().get_open_positions();
            if !positions.is_empty() {
                println!("Open position: {:?} {} units at {:.2}", 
                    positions[0].side, positions[0].quantity, positions[0].entry_price);
            }
        }
    }
    
    // Force close any pending candle
    if let Some(last_tick) = ticks.last() {
        tick_wrapper.force_close_candle(last_tick.timestamp * 1000);
        if tick_wrapper.get_current_candle().is_some() {
            candle_count += 1;
        }
    }
    
    // Print final results
    println!("\n=== Final Results ===");
    println!("Total candles formed: {}", candle_count);
    
    let trades = tick_wrapper.strategy().get_trades();
    println!("Total trades: {}", trades.len());
    
    let final_price = ticks.last().map(|t| t.price).unwrap_or(3000.0);
    let final_equity = tick_wrapper.strategy().calculate_equity(final_price);
    let pnl = final_equity - initial_cash;
    let pnl_pct = (pnl / initial_cash) * 100.0;
    
    println!("Initial cash: ${:.2}", initial_cash);
    println!("Final equity: ${:.2}", final_equity);
    println!("P&L: ${:.2} ({:.2}%)", pnl, pnl_pct);
    
    // Print all trades
    if !trades.is_empty() {
        println!("\n=== Trade History ===");
        for (i, trade) in trades.iter().enumerate() {
            println!("\nTrade {}: {:?}", i + 1, trade.side);
            println!("  Entry: ${:.2} @ timestamp {}", trade.entry_price, trade.entry_time);
            println!("  Exit:  ${:.2} @ timestamp {}", trade.exit_price, trade.exit_time);
            println!("  Quantity: {}", trade.quantity);
            println!("  P&L: ${:.2}", trade.pnl);
        }
    } else {
        println!("\nNo trades were generated!");
        println!("Possible reasons:");
        println!("- Not enough candles formed (only {} candles from {} ticks)", candle_count, ticks.len());
        println!("- Stochastic conditions not met (need K < 20 with crossover for buy)");
        println!("- Try adjusting the timeframe or strategy parameters");
    }
    
    // Show final indicator values
    println!("\n=== Final Indicator Status ===");
    if let (Some(k), Some(d)) = (tick_wrapper.strategy().current_k, tick_wrapper.strategy().current_d) {
        println!("Final Stochastic K: {:.2}, D: {:.2}", k, d);
    } else {
        println!("Stochastic indicators not initialized (need at least {} candles)", 
            tick_wrapper.strategy().config.k_period);
    }
}