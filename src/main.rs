use trading_strategies::Strategy;
use trading_strategies::core::{ProposedTrade, TradeDecision, TradeEvent, TradeObserver};
use trading_strategies::core::tick::TickData;
use trading_strategies::core::tick_strategy::TickStrategyWrapper;
use trading_strategies::strategies::config::{StochasticConfig, RSIConfig};
use trading_strategies::strategies::stochastic::StochasticStrategy;
use trading_strategies::strategies::rsi::{RSIStrategy, RsiTradeContext};
use chrono;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{BufRead, BufReader};
use serde::{Deserialize, Serialize};
use std::any::Any;

// Tick data structure matching the JSONL format
#[derive(Debug, Clone, Deserialize, Serialize)]
struct MarketTick {
    timestamp: i64,
    price: f64,
    volume: f64,
}

impl TickData for MarketTick {
    fn timestamp(&self) -> i64 { self.timestamp }
    fn price(&self) -> f64 { self.price }
    fn volume(&self) -> f64 { self.volume }
    fn symbol(&self) -> &str { "BTCUSDT" }
}

// Shared counters for tracking hook activity
#[derive(Clone)]
struct HookCounters {
    trades_validated: Arc<Mutex<usize>>,
    trades_rejected: Arc<Mutex<usize>>,
    trades_modified: Arc<Mutex<usize>>,
    trades_executed: Arc<Mutex<usize>>,
}

impl HookCounters {
    fn new() -> Self {
        Self {
            trades_validated: Arc::new(Mutex::new(0)),
            trades_rejected: Arc::new(Mutex::new(0)),
            trades_modified: Arc::new(Mutex::new(0)),
            trades_executed: Arc::new(Mutex::new(0)),
        }
    }
}

// Example observer that validates and modifies trades
struct RiskManager {
    max_price: f64,
    max_position_size: f64,
    counters: HookCounters,
    trade_count: usize,
}

impl RiskManager {
    fn new(max_price: f64, max_position_size: f64, counters: HookCounters) -> Self {
        Self { 
            max_price, 
            max_position_size,
            counters,
            trade_count: 0,
        }
    }
}

impl TradeObserver for RiskManager {
    // Called BEFORE trade execution
    fn before_trade(&mut self, proposed: &ProposedTrade, _context: Option<&dyn Any>) -> TradeDecision {
        self.trade_count += 1;
        let mut validated = self.counters.trades_validated.lock().unwrap();
        *validated += 1;
        let validation_num = *validated;
        drop(validated);
        
        println!("\n🔍 [PRE-TRADE HOOK] Validation #{}", validation_num);
        println!("   Timestamp: {}", chrono::Local::now().format("%H:%M:%S%.3f"));
        println!("   Proposed: {} {} units at ${:.2}", 
            match proposed.side {
                trading_strategies::core::Side::Long => "BUY",
                trading_strategies::core::Side::Short => "SELL",
            },
            proposed.quantity,
            proposed.price
        );
        
        // Validation 1: Check price limits
        if proposed.price > self.max_price {
            let mut rejected = self.counters.trades_rejected.lock().unwrap();
            *rejected += 1;
            println!("   ❌ REJECTED: Price ${:.2} exceeds limit ${:.2}", proposed.price, self.max_price);
            return TradeDecision::Reject("Price too high".to_string());
        }
        
        // Validation 2: Check if position size exceeds max
        if proposed.quantity > self.max_position_size {
            let mut modified = self.counters.trades_modified.lock().unwrap();
            *modified += 1;
            println!("   ⚠️  MODIFIED: Reducing size from {:.2} to {:.2} units", proposed.quantity, self.max_position_size);
            let mut modified_trade = proposed.clone();
            modified_trade.quantity = self.max_position_size;
            return TradeDecision::Modify(modified_trade);
        }
        
        // Validation 3: Dynamic risk management - reduce position size when price is near limit
        if proposed.price > 2950.0 && proposed.price <= self.max_price {
            // High price zone (2950-3000) - reduce position for risk management
            let risk_adjusted_size = proposed.quantity * 0.7; // Reduce by 30%
            if risk_adjusted_size < proposed.quantity {
                let mut modified = self.counters.trades_modified.lock().unwrap();
                *modified += 1;
                println!("   ⚠️  MODIFIED: High price zone (>${:.2}) - reducing size from {:.2} to {:.2} units for risk management", 
                    2950.0, proposed.quantity, risk_adjusted_size);
                let mut modified_trade = proposed.clone();
                modified_trade.quantity = risk_adjusted_size;
                return TradeDecision::Modify(modified_trade);
            }
        }
        
        println!("   ✅ APPROVED: Trade can proceed as proposed");
        TradeDecision::Approve
    }
    
    // Called AFTER trade execution
    fn post_trade(&mut self, event: TradeEvent, _context: Option<&dyn Any>) {
        let mut executed = self.counters.trades_executed.lock().unwrap();
        *executed += 1;
        let execution_num = *executed;
        drop(executed);
        
        println!("\n📊 [POST-TRADE HOOK] Execution #{}", execution_num);
        println!("   Timestamp: {}", chrono::Local::now().format("%H:%M:%S%.3f"));
        
        match event {
            TradeEvent::Buy(trade) => {
                println!("   Type: BUY ORDER EXECUTED");
                println!("   Details:");
                println!("     - Symbol: {}", trade.symbol);
                println!("     - Entry Price: ${:.2}", trade.entry_price);
                println!("     - Quantity: {} units", trade.quantity);
                println!("     - Total Cost: ${:.2}", trade.entry_price * trade.quantity);
                println!("     - Entry Time: {}", format_timestamp(trade.entry_time));
                println!("   💰 Position opened successfully");
            }
            TradeEvent::Sell(trade) => {
                println!("   Type: SELL ORDER EXECUTED");
                println!("   Details:");
                println!("     - Symbol: {}", trade.symbol);
                println!("     - Entry Price: ${:.2}", trade.entry_price);
                println!("     - Exit Price: ${:.2}", trade.exit_price);
                println!("     - Quantity: {} units", trade.quantity);
                println!("     - P&L: ${:.2} ({:.2}%)", trade.pnl, trade.pnl_percentage);
                println!("     - Entry Time: {}", format_timestamp(trade.entry_time));
                println!("     - Exit Time: {}", format_timestamp(trade.exit_time));
                println!("     - Duration: {} minutes", (trade.exit_time - trade.entry_time) / 60000);
                if trade.pnl > 0.0 {
                    println!("   ✅ Profitable trade!");
                } else {
                    println!("   ❌ Loss incurred");
                }
            }
        }
    }
}

// Context-aware observer that reads strategy-specific context
struct ContextAwareLogger;

impl TradeObserver for ContextAwareLogger {
    fn before_trade(&mut self, proposed: &ProposedTrade, context: Option<&dyn Any>) -> TradeDecision {
        println!("\n🔎 [CONTEXT-AWARE OBSERVER] Pre-trade analysis");
        println!("   Proposed: {} {} units at ${:.2}", 
            match proposed.side {
                trading_strategies::core::Side::Long => "BUY",
                trading_strategies::core::Side::Short => "SELL",
            },
            proposed.quantity,
            proposed.price
        );
        
        // Check if we have RSI context
        if let Some(rsi_context) = context.and_then(|ctx| ctx.downcast_ref::<RsiTradeContext>()) {
            println!("   📊 RSI Strategy Context:");
            println!("     - Current RSI: {:.2}", rsi_context.rsi_value);
            println!("     - Dynamic Oversold: {:.2}", rsi_context.dynamic_oversold);
            println!("     - Dynamic Overbought: {:.2}", rsi_context.dynamic_overbought);
            
            // Could use this for advanced validation
            if rsi_context.rsi_value < 20.0 {
                println!("     ⚠️  WARNING: Extremely oversold (RSI < 20)");
            } else if rsi_context.rsi_value > 80.0 {
                println!("     ⚠️  WARNING: Extremely overbought (RSI > 80)");
            }
        } else {
            println!("   ℹ️  No strategy-specific context provided");
        }
        
        TradeDecision::Approve
    }
    
    fn post_trade(&mut self, event: TradeEvent, context: Option<&dyn Any>) {
        match event {
            TradeEvent::Buy(_) => println!("   ✅ Buy trade executed"),
            TradeEvent::Sell(_) => println!("   ✅ Sell trade executed"),
        }
        
        if let Some(rsi_context) = context.and_then(|ctx| ctx.downcast_ref::<RsiTradeContext>()) {
            println!("   📊 Trade executed with RSI: {:.2}", rsi_context.rsi_value);
        }
    }
}

// Helper function to format timestamps
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};
    let secs = timestamp / 1000;
    let dt = DateTime::<Utc>::from_timestamp(secs, 0).unwrap();
    dt.format("%H:%M:%S").to_string()
}

// Load ticks from JSONL file
fn load_ticks(filename: &str) -> Result<Vec<MarketTick>, Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut ticks = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            let tick: MarketTick = serde_json::from_str(&line)?;
            ticks.push(tick);
        }
    }
    
    Ok(ticks)
}

fn main() {
    println!("=== Trading Observer Context Demo ===");
    println!("=====================================");
    println!("Demonstrating the new context parameter functionality");
    println!("that allows strategies to pass custom data to observers\n");
    
    // Load tick data
    let ticks = match load_ticks("stochastic_hooks_demo.jsonl") {
        Ok(ticks) => {
            println!("📁 Loaded {} ticks from stochastic_final_demo.jsonl", ticks.len());
            ticks
        }
        Err(e) => {
            eprintln!("❌ Error loading ticks: {}", e);
            return;
        }
    };
    
    // Create strategy configuration
    let config = StochasticConfig {
        k_period: 14,             // Standard stochastic period
        d_period: 3,              // Standard smoothing period
        oversold_threshold: 20.0, // Buy signal below this level
        overbought_threshold: 80.0, // Sell signal above this level
        position_size: 1.5,       // Base position size (can be modified by risk manager)
        atr_period: 14,
        atr_multiplier: 2.0,
    };
    
    // Create strategy wrapped for tick processing
    let strategy = StochasticStrategy::new(config, 10000.0);
    let mut tick_wrapper = TickStrategyWrapper::new(strategy, 10); // 10-second candles for more signals
    
    // Add risk manager with realistic limits
    println!("\n📋 Risk Manager Configuration:");
    println!("   - Max allowed price: $3,000.00");
    println!("   - Max position size: 1.0 unit");
    println!("   - Initial capital: $10,000.00");
    
    let counters = HookCounters::new();
    let risk_manager = RiskManager::new(3000.0, 1.0, counters.clone());
    tick_wrapper.strategy_mut().add_observer(Box::new(risk_manager));
    
    // Process ticks
    println!("\n📈 Market Simulation Starting...");
    println!("   Processing {} ticks", ticks.len());
    if let (Some(first), Some(last)) = (ticks.first(), ticks.last()) {
        println!("   Time range: {} to {}", 
            format_timestamp(first.timestamp), 
            format_timestamp(last.timestamp));
        println!("   Price range: ${:.2} to ${:.2}\n", 
            ticks.iter().map(|t| t.price).fold(f64::INFINITY, |a, b| a.min(b)),
            ticks.iter().map(|t| t.price).fold(f64::NEG_INFINITY, |a, b| a.max(b)));
    }
    
    // Track price for display
    let mut last_price = 0.0;
    let mut tick_count = 0;
    
    for tick in &ticks {
        tick_count += 1;
        
        // Show price movement periodically (every 10 ticks)
        if tick_count % 10 == 1 {
            if last_price == 0.0 {
                println!("⏱️  [{}] Price: ${:.2} (starting)", 
                    format_timestamp(tick.timestamp), tick.price);
            } else if tick.price > last_price {
                println!("⏱️  [{}] Price: ${:.2} ↗️  (+{:.2}%)", 
                    format_timestamp(tick.timestamp), 
                    tick.price,
                    ((tick.price - last_price) / last_price) * 100.0);
            } else if tick.price < last_price {
                println!("⏱️  [{}] Price: ${:.2} ↘️  ({:.2}%)", 
                    format_timestamp(tick.timestamp), 
                    tick.price,
                    ((tick.price - last_price) / last_price) * 100.0);
            } else {
                println!("⏱️  [{}] Price: ${:.2} →", 
                    format_timestamp(tick.timestamp), tick.price);
            }
            last_price = tick.price;
        }
        
        // Process the tick
        tick_wrapper.process_tick(tick);
    }
    
    // Force close any pending candle
    if let Some(last_tick) = ticks.last() {
        tick_wrapper.force_close_candle(last_tick.timestamp + 1000);
    }
    
    // Summary
    println!("\n╔══════════════════════════════════╗");
    println!("║        TRADING SUMMARY           ║");
    println!("╚══════════════════════════════════╝");
    
    let trades = tick_wrapper.strategy().get_trades();
    let final_price = ticks.last().map(|t| t.price).unwrap_or(100.0);
    let final_equity = tick_wrapper.strategy().calculate_equity(final_price);
    let total_pnl = final_equity - 10000.0;
    
    println!("📊 Performance Metrics:");
    println!("   - Initial Capital: $10,000.00");
    println!("   - Final Equity: ${:.2}", final_equity);
    println!("   - Total P&L: ${:.2} ({:.2}%)", total_pnl, (total_pnl / 10000.0) * 100.0);
    println!("   - Completed Trades: {}", trades.len());
    
    // Count winning/losing trades
    let winning_trades = trades.iter().filter(|t| t.pnl > 0.0).count();
    let losing_trades = trades.iter().filter(|t| t.pnl <= 0.0).count();
    
    println!("\n📈 Trade Statistics:");
    println!("   - Winning Trades: {}", winning_trades);
    println!("   - Losing Trades: {}", losing_trades);
    if !trades.is_empty() {
        println!("   - Win Rate: {:.1}%", (winning_trades as f64 / trades.len() as f64) * 100.0);
    }
    
    println!("\n🔍 Hook Activity Summary:");
    let validations = *counters.trades_validated.lock().unwrap();
    let rejections = *counters.trades_rejected.lock().unwrap();
    let modifications = *counters.trades_modified.lock().unwrap();
    let executions = *counters.trades_executed.lock().unwrap();
    
    println!("   - Pre-trade validations: {} total", validations);
    println!("     • Approved: {}", validations - rejections - modifications);
    println!("     • Modified: {}", modifications);
    println!("     • Rejected: {}", rejections);
    println!("   - Post-trade notifications: {} events", executions);
    
    println!("\n💡 Benefits Demonstrated:");
    println!("   ✓ Pre-trade validation prevents bad trades");
    println!("   ✓ Trade modification ensures risk limits");
    println!("   ✓ Post-trade tracking for audit trails");
    
    // Now demonstrate RSI strategy with context
    println!("\n\n╔══════════════════════════════════════════════╗");
    println!("║    RSI STRATEGY WITH CONTEXT DEMONSTRATION   ║");
    println!("╚══════════════════════════════════════════════╝");
    
    // Create RSI strategy configuration
    let rsi_config = RSIConfig {
        rsi_period: 14,
        oversold_threshold: 30.0,
        overbought_threshold: 70.0,
        position_size: 1.0,
        use_dynamic_levels: true,
        volatility_window: 20,
        overbought_min: 65.0,
        overbought_max: 85.0,
        oversold_min: 15.0,
        oversold_max: 35.0,
        atr_period: 14,
        atr_multiplier: 2.0,
    };
    
    // Create RSI strategy with context-aware observer
    let rsi_strategy = RSIStrategy::new(rsi_config, 10000.0);
    let mut rsi_tick_wrapper = TickStrategyWrapper::new(rsi_strategy, 10);
    
    // Add context-aware observer
    println!("\n📋 Adding Context-Aware Observer to RSI Strategy");
    println!("   This observer will receive RSI-specific context data");
    rsi_tick_wrapper.strategy_mut().add_observer(Box::new(ContextAwareLogger));
    
    // Process all ticks for demonstration to ensure RSI has enough data
    println!("\n📈 Processing ticks through RSI strategy...");
    let demo_ticks = &ticks;
    
    for tick in demo_ticks {
        rsi_tick_wrapper.process_tick(tick);
    }
    
    // Force close any pending candle
    if let Some(last_tick) = demo_ticks.last() {
        rsi_tick_wrapper.force_close_candle(last_tick.timestamp + 1000);
    }
    
    // Summary of context demonstration
    println!("\n📊 Context Demonstration Summary:");
    println!("   - Stochastic Strategy: Passed None as context");
    println!("   - RSI Strategy: Passed RsiTradeContext with:");
    println!("     • Current RSI value");
    println!("     • Dynamic overbought threshold");
    println!("     • Dynamic oversold threshold");
    println!("\n✨ This allows observers to make more informed decisions");
    println!("   based on strategy-specific internal state!");
}