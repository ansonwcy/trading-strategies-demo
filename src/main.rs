use trading_strategies::{Strategy, CandleData};
use trading_strategies::core::{ProposedTrade, TradeDecision, TradeEvent, TradeObserver};
use trading_strategies::strategies::config::MovingAverageConfig;
use trading_strategies::strategies::moving_average::MovingAverageStrategy;

// Simple candle implementation
struct SimpleCandle {
    price: f64,
    timestamp: i64,
}

impl CandleData for SimpleCandle {
    fn open(&self) -> f64 { self.price }
    fn high(&self) -> f64 { self.price + 1.0 }
    fn low(&self) -> f64 { self.price - 1.0 }
    fn close(&self) -> f64 { self.price }
    fn volume(&self) -> f64 { 1000.0 }
    fn timestamp(&self) -> i64 { self.timestamp }
}

// Example observer that validates and modifies trades
struct RiskManager {
    max_price: f64,
    max_position_size: f64,
    trades_validated: usize,
    trades_rejected: usize,
    trades_modified: usize,
}

impl RiskManager {
    fn new(max_price: f64, max_position_size: f64) -> Self {
        Self { 
            max_price, 
            max_position_size,
            trades_validated: 0,
            trades_rejected: 0,
            trades_modified: 0,
        }
    }
}

impl TradeObserver for RiskManager {
    // Called BEFORE trade execution
    fn before_trade(&mut self, proposed: &ProposedTrade) -> TradeDecision {
        self.trades_validated += 1;
        
        println!("\n🔍 Pre-trade Validation #{}", self.trades_validated);
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
            self.trades_rejected += 1;
            println!("   ❌ REJECTED: Price exceeds ${:.2} limit", self.max_price);
            return TradeDecision::Reject("Price too high".to_string());
        }
        
        // Validation 2: Adjust position size if needed
        if proposed.quantity > self.max_position_size {
            self.trades_modified += 1;
            println!("   ⚠️  MODIFIED: Reducing size to {} units", self.max_position_size);
            let mut modified = proposed.clone();
            modified.quantity = self.max_position_size;
            return TradeDecision::Modify(modified);
        }
        
        println!("   ✅ APPROVED");
        TradeDecision::Approve
    }
    
    // Called AFTER trade execution
    fn post_trade(&mut self, event: TradeEvent) {
        match event {
            TradeEvent::Buy(trade) => {
                println!("   → Executed: Bought {} units at ${:.2}", 
                    trade.quantity, trade.entry_price);
            }
            TradeEvent::Sell(trade) => {
                println!("   → Executed: Sold at ${:.2}, P&L: ${:.2}", 
                    trade.exit_price, trade.pnl);
            }
        }
    }
}

fn main() {
    println!("=== Pre-trade Hooks Demo ===\n");
    
    // Create strategy that tries to buy 2 units
    let config = MovingAverageConfig {
        fast_period: 3,
        slow_period: 5,
        position_size: 2.0,  // Will try to buy 2 units
        min_separation_pct: 0.01,
        min_bars_since_cross: 0,
        use_volume_confirmation: false,
        volume_surge_threshold: 1.5,
        atr_period: 14,
        atr_multiplier: 2.0,
    };
    
    let mut strategy = MovingAverageStrategy::new(config, 10000.0);
    
    // Add risk manager that limits trades
    println!("Risk Manager Settings:");
    println!("- Max price: $107.00");
    println!("- Max position size: 1.0 unit");
    
    let risk_manager = RiskManager::new(107.0, 1.0);
    strategy.add_observer(Box::new(risk_manager));
    
    // Simulate market data
    let prices = vec![
        // Downtrend
        105.0, 104.0, 103.0, 102.0, 101.0, 100.0,
        // Uptrend - triggers BUY (will be modified)
        101.0, 102.0, 103.0, 104.0, 105.0, 106.0,
        // High prices - triggers another BUY (will be rejected)
        107.0, 108.0, 109.0, 110.0,
        // Downtrend - triggers SELL
        109.0, 108.0, 107.0, 106.0, 105.0, 104.0, 103.0,
    ];
    
    println!("\n📊 Processing {} price points...", prices.len());
    
    for (i, &price) in prices.iter().enumerate() {
        let candle = SimpleCandle {
            price,
            timestamp: (i as i64) * 60000,
        };
        strategy.process_candle(&candle);
    }
    
    // Summary
    println!("\n=== Summary ===");
    println!("Completed trades: {}", strategy.get_trades().len());
    let final_equity = strategy.calculate_equity(*prices.last().unwrap_or(&100.0));
    println!("Final P&L: ${:.2}", final_equity - 10000.0);
    
    println!("\n=== Pre-trade Hook Benefits ===");
    println!("✓ Validate trades before execution");
    println!("✓ Enforce risk limits automatically");
    println!("✓ Modify trade parameters on the fly");
    println!("✓ Integrate with external systems");
    println!("✓ No changes needed to strategy logic");
}