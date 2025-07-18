use trading_strategies::Strategy;
use trading_strategies::core::{ProposedTrade, TradeDecision, TradeEvent, TradeObserver};
use trading_strategies::core::types::TradeContext;
use trading_strategies::core::tick::TickData;
use trading_strategies::core::tick_strategy::TickStrategyWrapper;
use trading_strategies::strategies::config::RSIConfig;
use trading_strategies::strategies::rsi::{RSIStrategy, RsiTradeContext};
use std::fs::File;
use std::io::{BufRead, BufReader};
use serde::{Deserialize, Serialize};

// Simple tick data structure
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

// Simple custom data for demo
#[derive(Debug, Clone)]
struct TradeMetadata {
    user_id: String,
    session_id: String,
    risk_level: String,
}

// Demo 1: Pre-trade hooks (modify, reject, approve)
struct PreTradeHookDemo {
    max_position_size: f64,
    rejected_count: usize,
    modified_count: usize,
    approved_count: usize,
}

impl PreTradeHookDemo {
    fn new(max_position_size: f64) -> Self {
        Self {
            max_position_size,
            rejected_count: 0,
            modified_count: 0,
            approved_count: 0,
        }
    }
}

impl TradeObserver for PreTradeHookDemo {
    fn pre_trade(&mut self, proposed_trade: &ProposedTrade, _context: TradeContext) -> TradeDecision {
        println!("Pre-trade Hook: Evaluating trade at ${:.2}, size: {:.2}", 
                 proposed_trade.price, proposed_trade.quantity);

        // Rule 1: Reject trades above $50,000
        if proposed_trade.price > 50000.0 {
            self.rejected_count += 1;
            println!("  → REJECTED: Price too high (${:.2} > $50,000)", proposed_trade.price);
            return TradeDecision::Reject("Price too high".to_string());
        }

        // Rule 2: Modify position size if too large
        if proposed_trade.quantity > self.max_position_size {
            let old_size = proposed_trade.quantity;
            let mut modified_trade = proposed_trade.clone();
            modified_trade.quantity = self.max_position_size;
            self.modified_count += 1;
            println!("  → MODIFIED: Position size reduced from {:.2} to {:.2}", 
                     old_size, modified_trade.quantity);
            return TradeDecision::Modify(modified_trade);
        }

        // Rule 3: Approve normal trades
        self.approved_count += 1;
        println!("  → APPROVED: Trade looks good");
        TradeDecision::Approve
    }

    fn post_trade(&mut self, _event: TradeEvent, _context: TradeContext) {
        // No action needed for completed trades in this demo
    }
}

// Demo 2: Strategy context and custom data observer
struct ContextDemo {
    trade_count: usize,
}

impl ContextDemo {
    fn new() -> Self {
        Self { trade_count: 0 }
    }
}

impl TradeObserver for ContextDemo {
    fn pre_trade(&mut self, _proposed_trade: &ProposedTrade, _context: TradeContext) -> TradeDecision {
        TradeDecision::Approve
    }

    fn post_trade(&mut self, event: TradeEvent, context: TradeContext) {
        self.trade_count += 1;
        
        let (side, trade) = match event {
            TradeEvent::Buy(trade) => ("Long", trade),
            TradeEvent::Sell(trade) => ("Short", trade),
        };
        
        println!("Trade #{}: {} at ${:.2}", 
                 self.trade_count, side, trade.exit_price);

        // Show strategy context if available
        if let Some(rsi_context) = context.strategy_context
            .and_then(|ctx| ctx.downcast_ref::<RsiTradeContext>()) {
            println!("  Strategy Context:");
            println!("    RSI Value: {:.2}", rsi_context.rsi_value);
            println!("    Overbought Level: {:.2}", rsi_context.dynamic_overbought);
            println!("    Oversold Level: {:.2}", rsi_context.dynamic_oversold);
        }

        // Show custom data if available
        if let Some(custom_data) = context.custom_data {
            if let Some(metadata) = custom_data.downcast_ref::<TradeMetadata>() {
                println!("  Custom Data:");
                println!("    User ID: {}", metadata.user_id);
                println!("    Session ID: {}", metadata.session_id);
                println!("    Risk Level: {}", metadata.risk_level);
            }
        }
        println!();
    }
}

fn load_ticks() -> Vec<MarketTick> {
    let file_path = "stochastic_hooks_demo.jsonl";
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => {
            println!("Warning: Could not load {}, using sample data", file_path);
            return create_sample_ticks();
        }
    };

    let reader = BufReader::new(file);
    let mut ticks = Vec::new();
    
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(tick) = serde_json::from_str::<MarketTick>(&line) {
                ticks.push(tick);
            }
        }
    }
    
    if ticks.is_empty() {
        create_sample_ticks()
    } else {
        ticks
    }
}

fn create_sample_ticks() -> Vec<MarketTick> {
    vec![
        MarketTick { timestamp: 1000, price: 45000.0, volume: 1.0 },
        MarketTick { timestamp: 2000, price: 46000.0, volume: 1.5 },
        MarketTick { timestamp: 3000, price: 47000.0, volume: 2.0 },
        MarketTick { timestamp: 4000, price: 51000.0, volume: 1.0 }, // High price - should be rejected
        MarketTick { timestamp: 5000, price: 48000.0, volume: 3.0 },
        MarketTick { timestamp: 6000, price: 49000.0, volume: 2.5 },
        MarketTick { timestamp: 7000, price: 47500.0, volume: 1.8 },
        MarketTick { timestamp: 8000, price: 46500.0, volume: 2.2 },
    ]
}

fn main() {
    println!("=== Trading Hooks Demo ===\n");

    let ticks = load_ticks();
    println!("Loaded {} ticks for demo\n", ticks.len());

    // Demo 1: Pre-trade hooks (modify, reject, approve)
    println!("--- Demo 1: Pre-trade Hooks ---");
    demo_pre_trade_hooks(&ticks);

    println!("\n{}\n", "=".repeat(50));

    // Demo 2: Strategy context and custom data
    println!("--- Demo 2: Strategy Context & Custom Data ---");
    demo_strategy_context(&ticks);
}

fn demo_pre_trade_hooks(ticks: &[MarketTick]) {
    println!("Testing pre-trade hooks: modify, reject, approve trades\n");

    let config = RSIConfig {
        rsi_period: 14,
        oversold_threshold: 30.0,
        overbought_threshold: 70.0,
        position_size: 3.0, // Large size to trigger modifications
        use_dynamic_levels: false,
        volatility_window: 20,
        overbought_min: 65.0,
        overbought_max: 85.0,
        oversold_min: 15.0,
        oversold_max: 35.0,
        atr_period: 14,
        atr_multiplier: 2.0,
    };

    let rsi_strategy = RSIStrategy::new(config, 100000.0);
    let mut rsi_wrapper = TickStrategyWrapper::new(rsi_strategy, 5);

    // Add pre-trade hook observer
    let hook_observer = PreTradeHookDemo::new(2.0); // Max position size: 2.0
    rsi_wrapper.strategy_mut().add_observer(Box::new(hook_observer));

    println!("Processing ticks...\n");
    for tick in ticks {
        rsi_wrapper.process_tick(tick, None);
    }

    if let Some(last_tick) = ticks.last() {
        rsi_wrapper.force_close_candle(last_tick.timestamp + 1000);
    }

    println!("\nPre-trade Hook Results:");
    println!("  Note: Observer results would be tracked separately in a real implementation");
    println!("  Check console output above for individual trade decisions");
}

fn demo_strategy_context(ticks: &[MarketTick]) {
    println!("Testing strategy context and custom data flow\n");

    let config = RSIConfig {
        rsi_period: 14,
        oversold_threshold: 30.0,
        overbought_threshold: 70.0,
        position_size: 1.0,
        use_dynamic_levels: true, // Enable dynamic levels for context
        volatility_window: 20,
        overbought_min: 65.0,
        overbought_max: 85.0,
        oversold_min: 15.0,
        oversold_max: 35.0,
        atr_period: 14,
        atr_multiplier: 2.0,
    };

    let rsi_strategy = RSIStrategy::new(config, 100000.0);
    let mut rsi_wrapper = TickStrategyWrapper::new(rsi_strategy, 5);

    // Add context observer
    let context_observer = ContextDemo::new();
    rsi_wrapper.strategy_mut().add_observer(Box::new(context_observer));

    // Create different custom data for different users/sessions
    let users = ["alice", "bob", "charlie", "david", "eve"];
    let risk_levels = ["low", "medium", "high", "medium", "low"];
    let mut user_index = 0;
    let mut session_counter = 1000;

    println!("Processing ticks with custom data...\n");
    for (i, tick) in ticks.iter().enumerate() {
        // Change user every 20 ticks
        if i % 20 == 0 && i > 0 {
            user_index = (user_index + 1) % users.len();
            session_counter += 1;
        }
        
        // Create dynamic custom data
        let custom_data = TradeMetadata {
            user_id: format!("user_{}", users[user_index]),
            session_id: format!("session_{}", session_counter),
            risk_level: risk_levels[user_index].to_string(),
        };
        
        rsi_wrapper.process_tick(tick, Some(&custom_data));
    }

    if let Some(last_tick) = ticks.last() {
        // Use the last user's data for the final candle close
        let final_custom_data = TradeMetadata {
            user_id: format!("user_{}", users[user_index]),
            session_id: format!("session_{}", session_counter),
            risk_level: risk_levels[user_index].to_string(),
        };
        rsi_wrapper.force_close_candle_with_custom_data(last_tick.timestamp + 1000, Some(&final_custom_data));
    }

    let trades = rsi_wrapper.strategy().get_trades();
    println!("Total trades executed: {}", trades.len());
}